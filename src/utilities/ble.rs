use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    gatt, Ble, HciConnector,
};

use esp32_hal::{clock::Clocks, peripherals::*, prelude::*, Delay, IO};
use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{ble::controller::BleConnector, EspWifiInitialization};

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

pub fn connection(init: EspWifiInitialization, mut bluetooth: BT, io: IO, clocks: Clocks<'_>) -> ! {
    let mut delay = Delay::new(&clocks);
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
    let mut debounce_cnt = 500;

    loop {
        let connector = BleConnector::new(&init, &mut bluetooth);
        let hci: HciConnector<BleConnector<'_>> =
            HciConnector::new(connector, esp_wifi::current_millis);
        let mut ble = Ble::new(&hci);
        println!("{:?}", ble.init());
        println!("{:?}", ble.cmd_set_le_advertising_parameters());
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
        );
        println!("{:?}", ble.cmd_set_le_advertise_enable(true));
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

        loop {
            let mut notification = None;

            if button.is_low().unwrap() && debounce_cnt > 0 {
                debounce_cnt -= 1;
                println!("{}", debounce_cnt);
                if debounce_cnt == 0 {
                    let mut cccd = [0u8; 1];
                    if let Some(1) = srv.get_characteristic_value(
                        my_characteristic_notify_enable_handle,
                        0,
                        &mut cccd,
                    ) {
                        // if notifications enabled
                        if cccd[0] == 1 {
                            notification = Some(NotificationData::new(
                                my_characteristic_handle,
                                &b"Notification"[..],
                            ));
                        }
                    }
                }
            };

            if button.is_high().unwrap() {
                debounce_cnt = 500;
            }

            match srv.do_work_with_notification(notification) {
                Ok(res) => {
                    if let WorkResult::GotDisconnected = res {
                        break;
                    }
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
            delay.delay_ms(1000u32);
        }
    }
}
