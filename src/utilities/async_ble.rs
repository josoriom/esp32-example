#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_closure)]

use core::{cell::RefCell, pin::Pin};

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    async_attribute_server::AttributeServer,
    asynch::Ble,
    attribute_server::NotificationData,
    gatt,
};
use embassy_executor::Spawner;
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{
    ble::controller::asynch::BleConnector, initialize, EspWifiInitFor, EspWifiInitialization,
};

use esp32_hal as hal;
use hal::{
    clock::ClockControl, embassy, peripheral::Peripheral, peripherals::*, prelude::*,
    timer::TimerGroup, Rng, IO,
};

pub async fn connection(
    device_name: &str,
    init: EspWifiInitialization,
    mut bluetooth: BT,
    pins: esp32_hal::gpio::Pins,
) -> ! {
    let connector = BleConnector::new(&init, &mut bluetooth);
    println!("Connector created");
    let mut ble = Ble::new(connector, esp_wifi::current_millis);
    let mut led = pins.gpio2.into_push_pull_output();
    loop {
        println!("{:?}", ble.init().await);
        println!("{:?}", ble.cmd_set_le_advertising_parameters().await);
        println!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                    AdStructure::CompleteLocalName(device_name),
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
            led.toggle();
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
            // TODO how to check if notifications are enabled for the characteristic?
            // maybe pass something into the closure which just can query the characterisic value
            // probably passing in the attribute server won't work?
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
