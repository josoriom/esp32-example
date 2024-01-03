use core::cell::RefCell;

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    async_attribute_server::AttributeServer,
    asynch::Ble,
    attribute_server::NotificationData,
    gatt,
};
use embedded_hal_async::digital::Wait;
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{ble::controller::asynch::BleConnector, EspWifiInitialization};

use esp32_hal::{clock::Clocks, embassy, peripherals::*, prelude::*, timer::TimerGroup, IO};

pub async fn connection(
    init: EspWifiInitialization,
    mut bluetooth: BT,
    io: IO,
    clocks: Clocks<'_>,
    timg0: TIMG0,
) -> ! {
    let pins = io.pins;
    #[cfg(any(feature = "esp32", feature = "esp32s2", feature = "esp32s3"))]
    let mut led = pins.gpio2.into_push_pull_output();
    let button = pins.gpio0.into_pull_down_input();
    #[cfg(any(
        feature = "esp32c2",
        feature = "esp32c3",
        feature = "esp32c6",
        feature = "esp32h2"
    ))]
    let button = pins.gpio9.into_pull_down_input();

    // Async requires the GPIO interrupt to wake futures
    esp32_hal::interrupt::enable(
        esp32_hal::peripherals::Interrupt::GPIO,
        esp32_hal::interrupt::Priority::Priority1,
    )
    .unwrap();

    let timer_group0 = TimerGroup::new(timg0, &clocks);
    embassy::init(&clocks, timer_group0.timer0);

    let connector = BleConnector::new(&init, &mut bluetooth);
    let mut ble: Ble<_> = Ble::new(connector, esp_wifi::current_millis);
    println!("Connector created");

    let pin_ref = RefCell::new(button);

    loop {
        println!("{:?}", ble.init().await);
        println!("{:?}", ble.cmd_set_le_advertising_parameters().await);
        println!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                    AdStructure::CompleteLocalName("ESP32"),
                ])
                .unwrap()
            )
            .await
        );
        println!("{:?}", ble.cmd_set_le_advertise_enable(true).await);

        println!("started advertising");

        let mut rf = |_offset: usize, data: &mut [u8]| {
            data[..5].copy_from_slice(&b"Hello!"[..]);
            5
        };
        let mut wf = |offset: usize, data: &[u8]| {
            println!("RECEIVED: {} {:?}", offset, data);
            let _ = led.toggle();
        };

        gatt!([service {
            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
            characteristics: [characteristic {
                name: "my_characteristic",
                uuid: "957312e0-2354-11eb-9f10-fbc30a62cf40",
                read: rf,
                write: wf,
                notify: true,
            }],
        }]);

        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes);

        let counter = RefCell::new(0u8);
        let mut notifier = async || {
            pin_ref.borrow_mut().wait_for_rising_edge().await.unwrap();
            let mut data = [0u8; 13];
            data.copy_from_slice(b"Notification0");
            {
                let mut counter = counter.borrow_mut();
                data[data.len() - 1] += *counter;
                *counter = (*counter + 1) % 10;
            }
            NotificationData::new(my_characteristic_handle, &data)
        };
        srv.run(&mut notifier).await.unwrap();
    }
}
