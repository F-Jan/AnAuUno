use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use anauuno::connection::{Connection, ConnectionContext};
use anauuno::stream::UsbAapStream;
use gstreamer::prelude::*;
use gstreamer_app::AppSrc;
use gstreamer_video::prelude::*;
use gstreamer_video::VideoOverlay;
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
    pipeline: Option<gstreamer::Pipeline>,
    appsink: Option<gstreamer_app::AppSink>,
    video_texture: Option<wgpu::Texture>,
    video_bind_group: Option<wgpu::BindGroup>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    video_width: u32,
    video_height: u32,
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
            pipeline: None,
            appsink: None,
            video_texture: None,
            video_bind_group: None,
            render_pipeline: None,
            video_width: 0,
            video_height: 0,
        })
    }

    pub fn set_pipeline(&mut self, pipeline: gstreamer::Pipeline) {
        self.pipeline = Some(pipeline);
    }

    pub fn set_appsink(&mut self, appsink: gstreamer_app::AppSink) {
        self.appsink = Some(appsink);
        self.setup_render_pipeline();
    }

    fn setup_render_pipeline(&mut self) {
        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let texture_bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let render_pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            immediate_size: 0,
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        });

        self.render_pipeline = Some(render_pipeline);

        let _sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });

        // We will create the texture and bind group when we get the first frame or in resize
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;

            if let Some(pipeline) = &self.pipeline {
                if let Some(overlay) = pipeline.dynamic_cast_ref::<VideoOverlay>() {
                    overlay.expose();
                }
            }
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // We can't render unless the surface is configured
        if !self.is_surface_configured {
            return Ok(());
        }

        if let Some(appsink) = &self.appsink {
            while let Some(sample) = appsink.try_pull_sample(gstreamer::ClockTime::from_mseconds(0)) {
                let buffer = sample.buffer().unwrap();
                let caps = sample.caps().unwrap();
                let structure = caps.structure(0).unwrap();
                let width = structure.get::<i32>("width").unwrap() as u32;
                let height = structure.get::<i32>("height").unwrap() as u32;

                let map = buffer.map_readable().unwrap();
                let data = map.as_slice();

                if self.video_texture.is_none() || 
                   self.video_texture.as_ref().unwrap().width() != width || 
                   self.video_texture.as_ref().unwrap().height() != height {
                    
                    self.video_width = width;
                    self.video_height = height;
                    
                    let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Video Texture"),
                        size: wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8UnormSrgb,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    });

                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::MipmapFilterMode::Linear,
                        ..Default::default()
                    });

                    let texture_bind_group_layout = self.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                        entries: &[
                            wgpu::BindGroupLayoutEntry {
                                binding: 0,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Texture {
                                    multisampled: false,
                                    view_dimension: wgpu::TextureViewDimension::D2,
                                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                                },
                                count: None,
                            },
                            wgpu::BindGroupLayoutEntry {
                                binding: 1,
                                visibility: wgpu::ShaderStages::FRAGMENT,
                                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                                count: None,
                            },
                        ],
                        label: Some("texture_bind_group_layout"),
                    });

                    let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                        layout: &texture_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(&view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                        ],
                        label: Some("video_bind_group"),
                    });

                    self.video_texture = Some(texture);
                    self.video_bind_group = Some(bind_group);
                }

                if let Some(texture) = &self.video_texture {
                    self.queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &texture,
                            mip_level: 0,
                            origin: wgpu::Origin3d::ZERO,
                            aspect: wgpu::TextureAspect::All,
                        },
                        data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(4 * width),
                            rows_per_image: Some(height),
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }
        }

        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

            if let (Some(pipeline), Some(bind_group)) = (&self.render_pipeline, &self.video_bind_group) {
                if self.video_width > 0 && self.video_height > 0 {
                    let window_width = self.config.width as f32;
                    let window_height = self.config.height as f32;
                    let video_width = self.video_width as f32;
                    let video_height = self.video_height as f32;

                    let window_aspect = window_width / window_height;
                    let video_aspect = video_width / video_height;

                    let (viewport_width, viewport_height, x, y) = if window_aspect > video_aspect {
                        // Window is wider than video - Pillarbox
                        let viewport_width = window_height * video_aspect;
                        let x = (window_width - viewport_width) / 2.0;
                        (viewport_width, window_height, x, 0.0)
                    } else {
                        // Window is taller than video - Letterbox
                        let viewport_height = window_width / video_aspect;
                        let y = (window_height - viewport_height) / 2.0;
                        (window_width, viewport_height, 0.0, y)
                    };

                    render_pass.set_viewport(x, y, viewport_width, viewport_height, 0.0, 1.0);
                }

                render_pass.set_pipeline(pipeline);
                render_pass.set_bind_group(0, bind_group, &[]);
                render_pass.draw(0..3, 0..1);
            }
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
    receiver: Option<std::sync::mpsc::Receiver<Vec<u8>>>,
}

impl App {
    pub fn new(context: Arc<Mutex<ConnectionContext>>, receiver: std::sync::mpsc::Receiver<Vec<u8>>) -> Self {
        Self {
            state: None,
            context,
            receiver: Some(receiver),
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let receiver = self.receiver.take().expect("Receiver already taken");
        let mut state = pollster::block_on(State::new(window.clone(), Arc::clone(&self.context))).unwrap();

        use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle, RawWindowHandle};
        let window_handle = window.window_handle().expect("Failed to get window handle").as_raw();
        let _display_handle = window.display_handle().expect("Failed to get display handle").as_raw();

        let _handle = match window_handle {
            RawWindowHandle::Xlib(h) => h.window as usize,
            RawWindowHandle::Xcb(h) => h.window.get() as usize,
            RawWindowHandle::Wayland(h) => h.surface.as_ptr() as usize,
            _ => 0,
        };

        // We need to start GStreamer here so we can set the window handle.
        gstreamer::init().expect("GStreamer could not be initialized");

        let pipeline = gstreamer::parse::launch(
            "appsrc name=src is-live=true format=time ! h264parse ! decodebin ! videoconvert ! video/x-raw,format=RGBA ! appsink name=sink"
        ).expect("Failed to create pipeline").dynamic_cast::<gstreamer::Pipeline>().unwrap();

        let appsrc = pipeline
            .clone()
            .dynamic_cast::<gstreamer::Bin>()
            .unwrap()
            .by_name("src")
            .expect("Source element not found")
            .dynamic_cast::<AppSrc>()
            .expect("Source element is not an AppSrc");

        let appsink = pipeline
            .clone()
            .dynamic_cast::<gstreamer::Bin>()
            .unwrap()
            .by_name("sink")
            .expect("Sink element not found")
            .dynamic_cast::<gstreamer_app::AppSink>()
            .expect("Sink element is not an AppSink");

        state.set_pipeline(pipeline.clone());
        state.set_appsink(appsink);

        pipeline.set_state(gstreamer::State::Playing).expect("Unable to set the pipeline to the `Playing` state");

        thread::spawn(move || {
            println!("Starting stream thread...");

            for data in receiver {
                // println!("Received buffer of size: {}", data.len());
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

        self.state = Some(state);
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

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
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

fn media_data_handler(_data: Data<u32>, _message: Message) {
    
}

fn main() -> rusb::Result<()> {
    let context = Context::new()?;
    // Hier Vendor und Product ID des normalen Android-Geräts (OEM‑IDs) einsetzen
    let _target_vid = 0x2717; // Xiaomi
    let _target_pid = 0xff08; // PID im normalen Modus

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
    let mut connection = Connection::new(stream, sender, Arc::clone(&context));

    let mut media_service = MediaSinkService::new(MediaSinkServiceConfig {});
    media_service.add_media_data_handler(media_data_handler);

    thread::spawn(move || {
        connection.start();
    });


    let event_loop = EventLoop::with_user_event().build().unwrap();
    let mut app = App::new(context, receiver);

    event_loop.run_app(&mut app).unwrap();


    Ok(())
}
