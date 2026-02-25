use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use anauuno::connection::{AapConnection, ConnectionContext};
use anauuno::stream::UsbAapStream;
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use rusb::{Context, Device, DeviceHandle, Hotplug, HotplugBuilder, UsbContext};
use std::thread;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window};
use anauuno::data::Data;
use anauuno::message::Message;
use anauuno::service::{MediaSinkService, MediaSinkServiceConfig};
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


// This will store the state of our game
pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    context: Arc<Mutex<ConnectionContext>>,
}

impl State {
    // We don't need this to be async right now,
    // but we will in the next tutorial
    pub async fn new(window: Arc<Window>, context: Arc<Mutex<ConnectionContext>>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            context,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {

        let key_code_id = match code {
            KeyCode::KeyH => 3,
            KeyCode::KeyB => 4,

            KeyCode::ArrowUp => 19,
            KeyCode::ArrowDown => 20,
            KeyCode::ArrowLeft => 21,
            KeyCode::ArrowRight => 22,
            KeyCode::Enter => 23,

            KeyCode::Digit1 => {
                if is_pressed {
                    let context = Arc::clone(&self.context);
                    let mut context = context.lock().unwrap();

                    context.commands().send_rotary_event(-1);
                }

                0
            },
            KeyCode::Digit2 => {
                if is_pressed {
                    let context = Arc::clone(&self.context);
                    let mut context = context.lock().unwrap();

                    context.commands().send_rotary_event(1);
                }

                0
            },
            _ => 0,
        };

        if key_code_id != 0 {
            //println!("Key {:?} pressed: {}", code, is_pressed);

            let context = Arc::clone(&self.context);
            let mut context = context.lock().unwrap();

            context.commands().send_key_event(key_code_id, is_pressed);
        }

        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    fn update(&mut self) {
        // remove `todo!()`
    }
}

pub struct App {
    state: Option<State>,
    context: Arc<Mutex<ConnectionContext>>,
}

impl App {
    pub fn new(context: Arc<Mutex<ConnectionContext>>) -> Self {
        Self {
            state: None,
            context,
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(pollster::block_on(State::new(window, Arc::clone(&self.context))).unwrap());
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        // This is where proxy.send_event() ends up
        self.state = Some(event);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: key_state,
                    ..
                },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}







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

    sender: Arc<Sender<DeviceHandle<Context>>>,
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

        let connection_sender = self.sender.clone();

        if desc.vendor_id() == self.aoa_vid && desc.product_id() == self.aoa_pid {
            thread::spawn(move || {
                let handle = device.open().unwrap();
                println!("Found AOA device VID=0x{:04x} PID=0x{:04x}", desc.vendor_id(), desc.product_id());

                if handle.kernel_driver_active(0).unwrap_or(false) {
                    handle.detach_kernel_driver(0).unwrap();
                }
                handle.claim_interface(0).unwrap();

                connection_sender.send(handle).unwrap();
            });

          return;
        }
    }

    fn device_left(&mut self, _device: Device<Context>) {}
}

fn media_data_handler(data: Data<u32>, message: Message) {
    
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

    let (sender, receiver) = std::sync::mpsc::channel::<DeviceHandle<Context>>();
    let usb_handler = USBHandler { device_vid: target_vid, device_pid: target_pid, aoa_vid: target_vid2, aoa_pid: target_pid2, sender: Arc::new(sender) };

    let _registration = HotplugBuilder::new()
        .enumerate(true)
        .register(context.clone(), Box::new(usb_handler)).expect("TODO: Error HotplugBuilder");

    thread::spawn(move || {
        loop {
            context.handle_events(None).unwrap();
        }
    });


    let handle = receiver.recv().unwrap();

    let (sender, receiver) = std::sync::mpsc::channel();
    //let (key_event_sender, key_event_receiver) = std::sync::mpsc::channel::<(u32, bool)>();
    let context = Arc::new(Mutex::new(ConnectionContext::new()));

    let stream = UsbAapStream::new(handle, 0x81, 0x01);
    let stream = OpenSSLTlsStream::new(stream);
    let mut connection = AapConnection::new(stream, sender, Arc::clone(&context));

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

    let mut media_service = MediaSinkService::new(MediaSinkServiceConfig {});
    media_service.add_media_data_handler(media_data_handler);

    thread::spawn(move || {
        connection.start();
    });


    let event_loop = EventLoop::with_user_event().build().unwrap();
    let mut app = App::new(context);

    event_loop.run_app(&mut app).unwrap();


    Ok(())
}
