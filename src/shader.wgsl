struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> VertexOutput {
    // Trick: Tạo tam giác khổng lồ bao phủ toàn màn hình
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    
    var out: VertexOutput;
    out.clip_position = vec4<f32>(pos[in_vertex_index], 0.0, 1.0);
    out.uv = pos[in_vertex_index];
    return out;
}

// Hàm SDF: Tính khoảng cách từ điểm p đến quả cầu bán kính 1.0 tại gốc tọa độ
fn map(p: vec3<f32>) -> f32 {
    return length(p) - 1.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Khởi tạo tia sáng
    let ro = vec3<f32>(0.0, 0.0, -3.0); // Camera lùi về sau 3 đơn vị
    // Hướng tia bắn từ camera qua pixel hiện tại trên canvas
    let rd = normalize(vec3<f32>(in.uv.x, in.uv.y, 1.0)); 

    var t = 0.0;
    var color = vec3<f32>(0.1, 0.1, 0.1); // Màu nền tối

    // Vòng lặp Ray Marching
    for (var i = 0; i < 64; i++) {
        let p = ro + rd * t;
        let d = map(p);
        
        if (d < 0.001) { // Đã chạm bề mặt quả cầu
            // Tạo màu sắc giả lập 3D dựa trên pháp tuyến (normal)
            let normal = normalize(p);
            color = normal * 0.5 + 0.5; // Ánh xạ từ [-1, 1] sang [0, 1] để làm màu
            break;
        }
        
        t += d;
        if (t > 10.0) { break; } // Bay quá xa
    }

    return vec4<f32>(color, 1.0);
}
