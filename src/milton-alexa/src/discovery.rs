use std::io;

/// The discovery task is all about responding to requests from alexa on a udp socket and returning
/// the information that it needs to actually send events.
pub async fn discovery(config: &crate::config::Config) -> io::Result<()> {
  log::info!("discovery task started");
  let socket = async_std::net::UdpSocket::bind("0.0.0.0:1900").await.map_err(|error| {
    log::warn!("unable to bind udb socket to port 1900");
    error
  })?;
  let mdns_addr = async_std::net::Ipv4Addr::new(239, 255, 255, 250);
  let interface = async_std::net::Ipv4Addr::new(0, 0, 0, 0);
  socket.set_broadcast(true)?;
  socket.join_multicast_v4(mdns_addr, interface)?;

  loop {
    let mut buf = vec![0u8; 1024];
    let (n, peer) = socket.recv_from(&mut buf).await?;

    if n == 0 {
      continue;
    }

    let packet = String::from_utf8_lossy(&buf[0..n]).to_lowercase();

    let is_discovery = packet.contains("man: \"ssdp:discover\"")
      || packet.contains("st: upnp:rootdevice")
      || packet.contains("st: urn:belkin:device:**")
      || packet.contains("st: ssdp:all");

    if !is_discovery {
      continue;
    }

    log::info!("has request from {peer:?} - {packet:?}");
    let date_str = chrono::offset::Utc::now().format("%a, %d %b %Y %H:%M:%S GMT");

    let response = format!(
      "HTTP/1.1 200 OK\r\n\
        CACHE-CONTROL: max-age=86400\r\n\
        DATE: {date_str}\r\n\
        EXT:\r\n\
        LOCATION: {}\r\n\
        OPT: \"http://schemas.upnp.org/upnp/1/0/\"; ns=01\r\n\
        01-NLS: {}\r\n\
        SERVER: {}\r\n\
        ST: ssdp:all\r\n\
        USN: {}\r\n\r\n",
      config.setup_location, config.device_id, config.server, config.usn,
    );

    socket.send_to(response.as_bytes(), &peer).await?;
  }
}
