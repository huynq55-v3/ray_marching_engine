use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// Hàm bất đồng bộ để khởi tạo GPU và chạy vòng lặp
async fn run() {
    env_logger::init();
    
    // 1. Khởi tạo vòng lặp sự kiện và cửa sổ
    let event_loop = EventLoop::new().unwrap();
    let window = std::sync::Arc::new(WindowBuilder::new()
        .with_title("Ray Marching Engine - Phase 1")
        .build(&event_loop)
        .unwrap());

    let size = window.inner_size();

    // 2. Khởi tạo WGPU
    // Instance là điểm bắt đầu. Backends::all() trên Ubuntu thường sẽ ưu tiên Vulkan.
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Tạo Surface (bề mặt vẽ liên kết với cửa sổ winit)
    let surface = instance.create_surface(window.clone()).unwrap();

    // Yêu cầu Adapter (Đại diện cho card đồ họa vật lý - Intel Iris Xe)
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("Không tìm thấy card đồ họa phù hợp!");

    // In ra thông tin card đồ họa để kiểm tra
    println!("Sử dụng card đồ họa: {:?}", adapter.get_info().name);

    // Yêu cầu Device (Giao diện logic) và Queue (Hàng đợi lệnh)
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    // Cấu hình Surface để biết cách hiển thị hình ảnh lên màn hình
    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats.iter()
        .copied()
        .find(|f| f.is_srgb())
        .unwrap_or(surface_caps.formats[0]);

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // 3. Vòng lặp chính của ứng dụng
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
                match event {
                    // Xử lý sự kiện đóng cửa sổ
                    WindowEvent::CloseRequested => elwt.exit(),
                    
                    WindowEvent::Resized(physical_size) => {
                        config.width = physical_size.width.max(1);
                        config.height = physical_size.height.max(1);
                        surface.configure(&device, &config);
                        window.request_redraw();
                    }
                    
                    // Vẽ lại màn hình
                    WindowEvent::RedrawRequested => {
                        let output = surface.get_current_texture().unwrap();
                        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

                        // Tạo bộ mã hóa lệnh (Command Encoder)
                        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Render Encoder"),
                        });

                        // Render Pass: Ra lệnh xóa màn hình bằng một màu cụ thể (màu xanh than)
                        {
                            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
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
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                        } // Render pass mượn encoder, block {} này giúp nhả mượn để dùng lệnh submit

                        // Gửi lệnh lên GPU và hiển thị
                        queue.submit(std::iter::once(encoder.finish()));
                        output.present();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                // Yêu cầu vẽ lại liên tục để chuẩn bị cho các frame sau này
                window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}

fn main() {
    // Sử dụng pollster để chạy hàm async run()
    pollster::block_on(run());
}
