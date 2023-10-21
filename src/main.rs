#![no_std]
#![no_main]

use esp_backtrace as _;
use esp_println::println;
use hal::{clock::ClockControl, gpio::GpioExt, peripherals::Peripherals, prelude::*, Delay};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let dp = peripherals;
    let system = dp.DPORT.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);
    let pins = dp.GPIO.split();
    let mut builtin_led = pins.gpio2.into_push_pull_output();
    println!("Hello world!");
    loop {
        println!("Loop...");
        builtin_led.set_high().ok();
        delay.delay_ms(1500u32);
        builtin_led.set_low().ok();
        delay.delay_ms(500u32);
    }
}
