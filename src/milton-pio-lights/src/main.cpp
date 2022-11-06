#include <Arduino.h>
#include "Adafruit_NeoPixel.h"

#define PIXELS_PIN A0

static Adafruit_NeoPixel pixels(9, PIXELS_PIN);
static char buffer[255] = {'\0'};
static uint32_t cursor = 0;
static uint32_t last_read = 0;
static bool cleared = false;

void process() {
  if (strcmp(buffer, "on") == 0) {
    Serial.println("ok (on)");
    pixels.fill(Adafruit_NeoPixel::Color(255, 255, 255));
    pixels.show();
    digitalWrite(LED_BUILTIN, HIGH);
    return;
  }

  if (strcmp(buffer, "red") == 0) {
    Serial.println("ok (red)");
    pixels.fill(Adafruit_NeoPixel::Color(255, 0, 0));
    pixels.show();
    return;
  }

  if (strcmp(buffer, "green") == 0) {
    Serial.println("ok (green)");
    pixels.fill(Adafruit_NeoPixel::Color(0, 255, 0));
    pixels.show();
    return;
  }

  if (strcmp(buffer, "blue") == 0) {
    Serial.println("ok (blue)");
    pixels.fill(Adafruit_NeoPixel::Color(0, 0, 255));
    pixels.show();
    return;
  }

  if (strcmp(buffer, "off") == 0) {
    Serial.println("ok (off)");
    pixels.fill(Adafruit_NeoPixel::Color(0, 0, 0));
    pixels.show();
    digitalWrite(LED_BUILTIN, LOW);
    return;
  }

  Serial.println("failed");
}

void setup(void) {
  pinMode(LED_BUILTIN, OUTPUT);
  pinMode(PIXELS_PIN, OUTPUT);
  pixels.begin();
  pixels.show();
  Serial.begin(115200);
}

void loop(void) {
  if (Serial.available() > 0) {
    char value = Serial.read();
    last_read = millis();
    cleared = false;

    if (value == '\n' || value == '\r') {
      process();
      cursor = 0;
      memset(buffer, '\0', 255);
      return;
    }

    buffer[cursor] = value;
    cursor ++;
  }

  if (millis() - last_read > 1000 && cursor != 0) {
    cursor = 0;

    if (!cleared) {
      cleared = true;
      Serial.println("error");
    }

    memset(buffer, '\0', 255);
  }
}
