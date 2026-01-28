use anauuno::connection::AapConnection;
use anauuno::stream::UsbAapStream;
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use rusb::{Context, Device, DeviceHandle, Hotplug, HotplugBuilder, UsbContext};
use std::thread;
use anauuno::tls::openssl::OpenSSLTlsStream;

// AOA‑Setup‑Requests
const ACCESSORY_GET_PROTOCOL: u8       = 51;
const ACCESSORY_SEND_STRING: u8        = 52;
const ACCESSORY_START: u8              = 53;

// String‑IDs für ACCESSORY_SEND_STRING
const STRING_MANUFACTURER: u16 = 0;
const STRING_MODEL: u16        = 1;
const STRING_DESCRIPTION: u16  = 2;
const STRING_VERSION: u16      = 3;
const STRING_URI: u16          = 4;
const STRING_SERIAL: u16       = 5;

// Beispiel‑Strings
const MANUFACTURER: &str = "Android";
const MODEL: &str        = "Android Auto";
const DESCRIPTION: &str  = "Android Auto";
const VERSION: &str      = "1.0";
const URI: &str          = "https://example.com";
const SERIAL: &str       = "HU-AAAAAA001";

fn enter_aoa_mode<T: UsbContext>(handle: &mut DeviceHandle<T>) -> rusb::Result<()> {
    // 1) Protokollversion erfragen
    let mut buf = [0u8; 2];
    handle.read_control(
        rusb::request_type(rusb::Direction::In, rusb::RequestType::Vendor, rusb::Recipient::Device),
        ACCESSORY_GET_PROTOCOL,
        0, 0,
        &mut buf,
        std::time::Duration::from_secs(1),
    )?;
    let protocol_version = u16::from_le_bytes(buf);
    println!("AOA Protocol Version: {}", protocol_version);

    // 2) Accessory‑Strings übertragen
    let send_string = |id: u16, s: &str| {
        handle.write_control(
            rusb::request_type(rusb::Direction::Out, rusb::RequestType::Vendor, rusb::Recipient::Device),
            ACCESSORY_SEND_STRING,
            0, id,
            s.as_bytes(),
            std::time::Duration::from_secs(1),
        )
    };

    send_string(STRING_MANUFACTURER, MANUFACTURER)?;
    send_string(STRING_MODEL, MODEL)?;
    send_string(STRING_DESCRIPTION, DESCRIPTION)?;
    send_string(STRING_VERSION, VERSION)?;
    send_string(STRING_URI, URI)?;
    send_string(STRING_SERIAL, SERIAL)?;
    println!("Accessory strings sent.");

    // 3) Accessory‑Modus starten
    handle.write_control(
        rusb::request_type(rusb::Direction::Out, rusb::RequestType::Vendor, rusb::Recipient::Device),
        ACCESSORY_START,
        0, 0,
        &[],
        std::time::Duration::from_secs(1),
    )?;
    println!("Sent ACCESSORY_START; device should re-enumerate.");

    Ok(())
}

struct USBHandler {
    device_vid: u16,
    device_pid: u16,
    aoa_vid: u16,
    aoa_pid: u16,
}


impl Hotplug<Context> for USBHandler {
    fn device_arrived(&mut self, device: Device<Context>) {
        let desc = device.device_descriptor().unwrap();

        println!("Device VID=0x{:04x} PID=0x{:04x}", desc.vendor_id(), desc.product_id());

        if desc.vendor_id() == self.device_vid && desc.product_id() == self.device_pid {
            thread::spawn(move || {
                match device.open() {
                    Ok(mut handle) => {
                        println!("Opening device for AOA mode...");
                        if let Err(e) = enter_aoa_mode(&mut handle) {
                            eprintln!("Error entering AOA mode: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Failed to open device: {}", e),
                }
            });

            return;
        }

        if desc.vendor_id() == self.aoa_vid && desc.product_id() == self.aoa_pid {
            thread::spawn(move || {
                let handle = device.open().unwrap();
                println!("Found AOA device VID=0x{:04x} PID=0x{:04x}", desc.vendor_id(), desc.product_id());

                if handle.kernel_driver_active(0).unwrap_or(false) {
                    handle.detach_kernel_driver(0).unwrap();
                }
                handle.claim_interface(0).unwrap();

                let (sender, receiver) = std::sync::mpsc::channel();

                let stream = UsbAapStream::new(handle, 0x81, 0x01);
                let stream = OpenSSLTlsStream::new(stream);
                let mut connection = AapConnection::new(stream, sender);


                thread::spawn(move || {
                    gstreamer::init().expect("GStreamer could not be initialized");

                    let pipeline = gstreamer::parse::launch(
                        "appsrc name=src is-live=true format=time ! h264parse ! decodebin ! autovideosink"
                    ).expect("Failed to create pipeline");

                    let appsrc = pipeline
                        .clone()
                        .dynamic_cast::<gstreamer::Bin>()
                        .unwrap()
                        .by_name("src")
                        .expect("Source element not found")
                        .dynamic_cast::<AppSrc>()
                        .expect("Source element is not an AppSrc");

                    pipeline.set_state(gstreamer::State::Playing).expect("Failed to set pipeline to Playing");

                    println!("Starting stream thread...");

                    for data in receiver {
                        let mut buffer = gstreamer::Buffer::with_size(data.len()).unwrap();
                        {
                            let buffer_ref = buffer.get_mut().unwrap();
                            buffer_ref.copy_from_slice(0, &data).unwrap();
                        }

                        if let Err(err) = appsrc.push_buffer(buffer) {
                            eprintln!("Error pushing buffer to appsrc: {}", err);
                            break;
                        }
                    }

                    pipeline.set_state(gstreamer::State::Null).ok();
                });


                connection.start();
            });

          return;
        }
    }

    fn device_left(&mut self, _device: Device<Context>) {
        // TODO: Handle device removal

        return;
    }
}

fn main() -> rusb::Result<()> {
    let context = Context::new()?;
    // Hier Vendor und Product ID des normalen Android-Geräts (OEM‑IDs) einsetzen
    let target_vid = 0x2717; // Xiaomi
    let target_pid = 0xff08; // PID im normalen Modus

    let target_vid = 0x0e8d; // Xiaomi
    let target_pid = 0x201c; // PID im normalen Modus


    let target_vid2 = 0x18d1; // Google
    let target_pid2 = 0x2d01; // AOA Modus


    let usb_handler = USBHandler { device_vid: target_vid, device_pid: target_pid, aoa_vid: target_vid2, aoa_pid: target_pid2 };

    let _registration = HotplugBuilder::new()
        .enumerate(true)
        .register(context.clone(), Box::new(usb_handler)).expect("TODO: Error HotplugBuilder");

    loop {
        context.handle_events(None)?;
    }

    Ok(())
}
