#![warn(clippy::missing_docs_in_private_items)]
#![no_std]
#![no_main]

//! This application leverages the usb peripheral and gpio output of an esp32c3 (specifically, the
//! xiao-esp32c3) to control the state of a ws2812 (ish?) led peripheral.

use esp32c3 as pac;
use esp32c3_hal as hal;

use hal::prelude::*;

use core::fmt::Write;
use core::ops::Deref;
use hal::{interrupt, utils::SmartLedsAdapter};
use riscv_rt::entry;
use smart_leds::SmartLedsWrite;

// -- CONFIG

// [todo] once we can `and_then` parse this into a const u8, lets do that.
/// Configuration value: how many leds are we working with
const LED_COUNT: usize = match option_env!("LED_COUNT") {
  Some(_) => 12,
  None => 1,
};

// -- Types

/// This type enumerates the possible states our LED control can be in.
#[derive(Default)]
enum LedState {
  /// When `Requesting`, we have received at least some serial data and are waiting to figure out
  /// how to interpret it.
  Requesting(([u8; 128], usize)),

  /// Once we have received a valid message, we'll move into the `Requested` state, with the
  /// concrete request kind included.
  Requested(milton_xiao::StateRequest),

  /// If we've encountered an error, make everything red, and blink on/off via countup.
  Failed(u8),

  /// When we are not recieving and have no pending request, we are idle/empty.
  #[default]
  Empty,
}

/// Define an alias that we'll use for our static variables that wraps the components of
/// `critial_section` and `core` that provide safety to interrupt/main boundary mutability.
type GlobalMut<T> = critical_section::Mutex<core::cell::RefCell<T>>;

/// Define the type alias that we are using four our `smart_leds` adapter up here. This definition
/// includes the amount of LEDs available at the hardware level.
#[allow(clippy::identity_op)]
type LightAdapter = SmartLedsAdapter<
  esp32c3_hal::pulse_control::ConfiguredChannel0,
  esp32c3_hal::gpio::GpioPin<
    hal::gpio::Unknown,
    hal::gpio::Bank0GpioRegisterAccess,
    hal::gpio::InputOutputAnalogPinType,
    2,
  >,
  { LED_COUNT * 24 + 1 }, // LED_COUNT * (channels * pulses) + 1
>;

/// This static holds the usb interface that we will use for writing and reading serial data.
static USB: GlobalMut<Option<hal::UsbSerialJtag<pac::USB_DEVICE>>> =
  critical_section::Mutex::new(core::cell::RefCell::new(None));

/// The gloal state wrapped in a mutex; this will be manipulated across interrupt/main.
static STATE: GlobalMut<LedState> = critical_section::Mutex::new(core::cell::RefCell::new(LedState::Empty));

/// A timer that will be reset across interrupt/main boundaries.
static PACKET_TIMER: GlobalMut<Option<hal::timer::Timer<hal::timer::Timer0<pac::TIMG0>>>> =
  critical_section::Mutex::new(core::cell::RefCell::new(None));

#[entry]
fn main() -> ! {
  // "Take ownership" of the registers at the svdtorust/pac level.
  let peripherals = pac::Peripherals::take().unwrap();

  // Create the hardware abstraction layer version from our peripheral access crate's raw register
  // struct definition;
  let mut system = hal::system::SystemExt::split(peripherals.SYSTEM);

  // Create the hardware abstraction layer for our real time clock control.
  let mut rtc = hal::Rtc::new(peripherals.RTC_CNTL);

  // Create hal IO layer from pac.
  let io = hal::IO::new(peripherals.GPIO, peripherals.IO_MUX);

  // Initialize the clock control.
  let clocks = hal::clock::ClockControl::boot_defaults(system.clock_control).freeze();

  // Create the usb serial abstraction.
  let mut usb_serial = hal::UsbSerialJtag::new(peripherals.USB_DEVICE);

  // We're not using either of these timers.
  let mut packet_timer = hal::timer::TimerGroup::new(peripherals.TIMG0, &clocks);
  let mut second_timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks);

  // Disable watchdog timers
  rtc.swd.disable();
  rtc.rwdt.disable();
  packet_timer.wdt.disable();
  second_timer.wdt.disable();

  // [todo-understand?] how is this different from the clocks we created earlier? meaning, what is
  // special about `system.peripheral_clock_control`.
  let pulse = hal::PulseControl::new(
    peripherals.RMT,
    &mut system.peripheral_clock_control,
    hal::pulse_control::ClockSource::APB,
    0,
    0,
    0,
  )
  .unwrap();

  // Create the adapter with a single LED.
  let mut led = LightAdapter::new(pulse.channel0, io.pins.gpio2);

  // Create our interrupt timer that will fail pending packets.
  packet_timer.timer0.start(30u64.millis());
  packet_timer.timer0.listen();

  critical_section::with(|cs| {
    PACKET_TIMER.borrow_ref_mut(cs).replace(packet_timer.timer0);
  });

  // Enable the usb serial interrupt
  usb_serial.listen_rx_packet_recv_interrupt();
  critical_section::with(|cs| USB.borrow_ref_mut(cs).replace(usb_serial));

  hal::interrupt::enable(pac::Interrupt::TG0_T0_LEVEL, hal::interrupt::Priority::Priority2).unwrap();
  hal::interrupt::enable(pac::Interrupt::USB_SERIAL_JTAG, hal::interrupt::Priority::Priority1).unwrap();
  hal::interrupt::set_kind(
    hal::Cpu::ProCpu,
    hal::interrupt::CpuInterrupt::Interrupt1,
    hal::interrupt::InterruptKind::Edge,
  );
  unsafe {
    riscv::interrupt::enable();
  }

  led.write(&mut [smart_leds::RGB8::new(0, 0, 0)].into_iter()).unwrap();

  loop {
    critical_section::with(|cs| {
      STATE.replace_with(cs, |state_reference| {
        match state_reference {
          // If we've received a 'off' request, turn the lights off.
          LedState::Requested(ref request) => {
            let mut colors = request.colors::<LED_COUNT>();
            match led.write(&mut colors) {
              Ok(_) => LedState::Empty,
              Err(_) => LedState::Failed(0),
            }
          }
          LedState::Failed(mut value) => {
            let color = if value % 2 == 0 {
              smart_leds::RGB8::new(255, 0, 0)
            } else {
              smart_leds::RGB8::new(0, 0, 0)
            };
            if value > 100 {
              value = 0;
            }
            match led.write(&mut [color].into_iter()) {
              Err(_) => LedState::Failed(value + 1),
              Ok(_) => LedState::Failed(value + 1),
            }
          }
          LedState::Requesting(_) | LedState::Empty => core::mem::take(state_reference),
        }
      });
    });
  }
}

/// Both the usb interrtup and the timeout interrupt will need to restart the packet timer. This
/// helper function really only exists to provide a thin layer of ergonomics and make sure we're
/// resetting it to the same interval.
fn restart_packet_timer<T>(timer: &mut Option<hal::timer::Timer<T>>) -> Option<()>
where
  T: esp32c3_hal::timer::Instance,
{
  if timer.is_none() {
    return None;
  }
  let timer = match timer.as_mut() {
    Some(t) => t,
    None => return None,
  };
  timer.clear_interrupt();
  timer.start(500u64.millis());
  Some(())
}

/// This interrupt is used to handle packet parsing timeout issues; if this macro fires and we are
/// currently in the process of recieving a packet, something has gone wrong.
#[hal::macros::interrupt]
fn TG0_T0_LEVEL() {
  critical_section::with(|cs| {
    // Access our timer and restart it.
    let mut timer = PACKET_TIMER.borrow_ref_mut(cs);
    restart_packet_timer(&mut *timer);

    // Check the state to see if we're currently in the middle of parsing a request.
    STATE.replace_with(cs, |state_reference| match state_reference {
      LedState::Requesting(_) => LedState::Failed(0),
      _ => LedState::Empty,
    });
  });
}

/// This interrupt fires whenever we have new data on our serial connection. It is responsible for
/// adding what is pending into our current buffer, or initializing a new one.
#[hal::macros::interrupt]
fn USB_SERIAL_JTAG() {
  // Immediately restart our timeout.
  critical_section::with(|cs| {
    let mut timer = PACKET_TIMER.borrow_ref_mut(cs);
    restart_packet_timer(&mut *timer);
  });

  critical_section::with(|cs| {
    // Update state, reading in every byte possible.
    let mut usb_serial = USB.borrow_ref_mut(cs);

    // Get a mutable reference, guarding against existing references by unwraping
    let usb_serial = match usb_serial.as_mut() {
      Some(u) => u,
      None => {
        STATE.replace_with(cs, |_| LedState::Failed(0));
        return;
      }
    };

    // Read bytes from our serial connection
    while let nb::Result::Ok(maybe_char) = usb_serial.read_byte() {
      STATE.replace_with(cs, |state_reference| match (maybe_char, state_reference.deref()) {
        // If we've reached a terminal character and are currently buffering, move our state into
        // the requested/failed based on a parse attempt.
        (b'\n', LedState::Requesting((buffer, cursor))) | (b':', LedState::Requesting((buffer, cursor))) => {
          if let Some(req) = milton_xiao::StateRequest::from_bytes(&buffer[0..*cursor]) {
            match write!(usb_serial, "{}", milton_xiao::Response::Roger) {
              Err(_) => LedState::Failed(0),
              Ok(_) => LedState::Requested(req),
            }
          } else {
            match write!(usb_serial, "{}", milton_xiao::Response::Failed) {
              Err(_) => LedState::Failed(0),
              Ok(_) => LedState::Failed(0),
            }
          }
        }

        // If we took a character and are buffering data, put it in there.
        (other, LedState::Requesting((mut buffer, cursor))) => {
          buffer[*cursor] = other;
          LedState::Requesting((buffer, cursor + 1))
        }

        // Otherwise, start buffering data.
        (other, _) => {
          let mut initial_buffer = [0; 128];
          initial_buffer[0] = other;
          LedState::Requesting((initial_buffer, 1))
        }
      });
    }

    // Ready the interrupt for the next byte.
    usb_serial.reset_rx_packet_recv_interrupt();
  });
}

/// [todo] We're using the JTAG/USB device for user input, so it is not clear what options are
/// available to us when we panic. Maybe we can store some stuff in flash or something.
#[panic_handler]
fn handle(_panic: &core::panic::PanicInfo) -> ! {
  loop {}
}
