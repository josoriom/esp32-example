#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]

use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{initialize, EspWifiInitFor};

use esp32_hal as hal;
use hal::{clock::ClockControl, peripherals::*, prelude::*, Rng, IO};

mod utilities {
    pub mod async_ble;
    pub mod ble;
}

use utilities::async_ble::connection;

#[main]
async fn main(_spawner: Spawner) -> ! {
    #[cfg(feature = "log")]
    esp_println::logger::init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();

    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();

    #[cfg(target_arch = "xtensa")]
    let timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    #[cfg(target_arch = "riscv32")]
    let timer = hal::systimer::SystemTimer::new(peripherals.SYSTIMER).alarm0;
    let init = initialize(
        EspWifiInitFor::Ble,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let bluetooth = peripherals.BT;
    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    connection(init, bluetooth, io, clocks, peripherals.TIMG0).await
}
