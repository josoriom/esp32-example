use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    gatt, Ble, HciConnector,
};

use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{ble::controller::BleConnector, initialize, EspWifiInitFor};
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Delay, Rng, IO};

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

pub fn connection(peripherals: Peripherals, name: &str) -> ! {
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::max(system.clock_control).freeze();
    let mut delay = Delay::new(&clocks);
    let pins = IO::new(peripherals.GPIO, peripherals.IO_MUX).pins;
    let mut button = pins.gpio0.into_pull_down_input();
    let mut led = pins.gpio2.into_push_pull_output();

    let timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    let init = initialize(
        EspWifiInitFor::Ble,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let mut bluetooth = peripherals.BT;

    let connector = BleConnector::new(&init, &mut bluetooth);
    let hci: HciConnector<BleConnector<'_>> =
        HciConnector::new(connector, esp_wifi::current_millis);
    let mut debounce_cnt = 500;

    loop {
        let mut ble = Ble::new(&hci);
        println!("{:?}", ble.init());
        println!("{:?}", ble.cmd_set_le_advertising_parameters());
        println!(
            "{:?}",
            ble.cmd_set_le_advertising_data(
                create_advertising_data(&[
                    AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                    AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                    AdStructure::CompleteLocalName(name),
                ])
                .unwrap()
            )
        );
        println!("{:?}", ble.cmd_set_le_advertise_enable(true));
        println!("{:?}", name);
        println!("started advertising");
        let mut rf = |_offset: usize, data: &mut [u8]| {
            data[..5].copy_from_slice(&b"Hello!"[..]);
            5
        };
        let mut wf = |offset: usize, data: &[u8]| {
            println!("RECEIVED: {} {:?}", offset, data);
            led.toggle();
        };

        #[allow(non_upper_case_globals)]
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
                println!("si esta presionado");
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
