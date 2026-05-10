use std::borrow::Cow;
use std::fs;
use std::sync::Arc;
use std::time::SystemTime;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

// Hàm tiện ích: Lấy thời gian sửa đổi cuối cùng của file
fn get_file_modified_time(path: &str) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

// Hàm cốt lõi: Đọc file WGSL và tạo ra Render Pipeline trên GPU
fn create_render_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    shader_path: &str,
) -> wgpu::RenderPipeline {
    // 1. Đọc code WGSL từ ổ cứng
    let shader_source = fs::read_to_string(shader_path).expect("Không thể đọc file shader!");

    // 2. Yêu cầu GPU biên dịch chuỗi text thành Shader Module
    let shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Ray Marching Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(&shader_source)),
    });

    // 3. Khởi tạo Pipeline Layout (Hiện tại để trống vì chưa truyền Uniform)
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    // 4. Lắp ráp Pipeline: Ghép Vertex, Fragment và Layout lại với nhau
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader_module,
            entry_point: "vs_main",
            buffers: &[], // Trống, vì ta dùng "trick" vertex_index
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader_module,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    })
}

async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = Arc::new(WindowBuilder::new()
        .with_title("Ray Marching Engine - Phase 2")
        .build(&event_loop)
        .unwrap());

    let size = window.inner_size();
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = instance.create_surface(window.clone()).unwrap();
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats.iter().find(|f| f.is_srgb()).unwrap_or(&surface_caps.formats[0]);
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: *surface_format,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let shader_path = "src/shader.wgsl";
    
    // Khởi tạo Pipeline lần đầu tiên
    let mut render_pipeline = create_render_pipeline(&device, config.format, shader_path);
    // Lưu lại thời điểm file được sửa lần cuối
    let mut last_modified = get_file_modified_time(shader_path);

    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { ref event, window_id } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        config.width = physical_size.width.max(1);
                        config.height = physical_size.height.max(1);
                        surface.configure(&device, &config);
                        window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        // KỸ THUẬT HOT-RELOAD: Kiểm tra xem file shader có bị sửa không?
                        if let Some(current_modified) = get_file_modified_time(shader_path) {
                            if Some(current_modified) != last_modified {
                                println!("Nhận diện thay đổi file! Đang nạp lại shader...");
                                // Chú ý: Ở phiên bản đơn giản này, nếu bạn gõ sai cú pháp WGSL, app sẽ crash do expect(). 
                                // Chúng ta sẽ học cách xử lý lỗi mềm sau.
                                render_pipeline = create_render_pipeline(&device, config.format, shader_path);
                                last_modified = Some(current_modified);
                            }
                        }

                        let output = surface.get_current_texture().unwrap();
                        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
                        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                        {
                            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("Render Pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), // Nền đen
                                        store: wgpu::StoreOp::Store,
                                    },
                                })],
                                depth_stencil_attachment: None,
                                timestamp_writes: None,
                                occlusion_query_set: None,
                            });
                            
                            // Gắn pipeline và vẽ 3 đỉnh của tam giác khổng lồ
                            render_pass.set_pipeline(&render_pipeline);
                            render_pass.draw(0..3, 0..1);
                        }

                        queue.submit(std::iter::once(encoder.finish()));
                        output.present();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                // Luôn yêu cầu vẽ lại liên tục để bắt sự kiện thay đổi file lập tức
                window.request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}

fn main() {
    pollster::block_on(run());
}
