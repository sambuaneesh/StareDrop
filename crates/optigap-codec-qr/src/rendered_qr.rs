use image::GrayImage;

pub fn render_luma_to_rgba(image: &GrayImage) -> Vec<u8> {
    let mut out = Vec::with_capacity((image.width() * image.height() * 4) as usize);
    for p in image.pixels() {
        let v = p[0];
        out.extend_from_slice(&[v, v, v, 255]);
    }
    out
}
