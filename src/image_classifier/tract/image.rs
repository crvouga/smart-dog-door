use image::{imageops, DynamicImage};
use tract_onnx::prelude::*;

pub fn resize_image(image: &DynamicImage, width: u32, height: u32) -> DynamicImage {
    if image.width() != image.height() {
        let (w, h) = (image.width() as f32, image.height() as f32);
        let scale = (width as f32 / w).min(height as f32 / h);
        let new_w = (w * scale) as u32;
        let new_h = (h * scale) as u32;

        let scaled = image.resize(new_w, new_h, imageops::FilterType::Triangle);

        let padded = DynamicImage::new_rgb8(width, height);
        let x_offset = (width - new_w) / 2;
        let y_offset = (height - new_h) / 2;

        let scaled_rgb = scaled.to_rgb8();
        let mut padded_rgb = padded.to_rgb8();

        let src_width = scaled_rgb.width();
        let src_height = scaled_rgb.height();

        for y in 0..new_h {
            for x in 0..new_w {
                if x < src_width && y < src_height {
                    let pixel = scaled_rgb.get_pixel(x, y);
                    padded_rgb.put_pixel(x + x_offset, y + y_offset, *pixel);
                }
            }
        }

        DynamicImage::from(padded_rgb)
    } else {
        image.resize_exact(width, height, imageops::FilterType::Triangle)
    }
}

fn image_to_tensor(
    image: &DynamicImage,
) -> Result<Tensor, Box<dyn std::error::Error + Send + Sync>> {
    let rgb = image.to_rgb8();
    let mut tensor = Tensor::zero::<f32>(&[1, 3, rgb.height() as usize, rgb.width() as usize])?;

    for c in 0..3 {
        for y in 0..rgb.height() {
            for x in 0..rgb.width() {
                let pixel = rgb.get_pixel(x, y);
                let index = c * (rgb.height() * rgb.width()) as usize
                    + y as usize * rgb.width() as usize
                    + x as usize;
                tensor.as_slice_mut::<f32>()?[index] = pixel[c] as f32 / 255.0;
            }
        }
    }

    Ok(tensor)
}

pub fn resize_image_to_tensor(
    image: &DynamicImage,
    width: u32,
    height: u32,
) -> Result<Tensor, Box<dyn std::error::Error + Send + Sync>> {
    let resized = resize_image(image, width, height);
    let tensor = image_to_tensor(&resized)?;

    Ok(tensor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, ImageBuffer, Rgb};

    #[test]
    fn test_image_to_tensor_square() {
        // Create a 100x100 red image
        let mut img = ImageBuffer::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([255, 0, 0]);
        }
        let image = DynamicImage::ImageRgb8(img);

        let tensor = resize_image_to_tensor(&image, 640, 640).unwrap();
        let shape = tensor.shape();
        assert_eq!(shape, &[1, 3, 640, 640]);

        // Check that the tensor contains the correct values
        let slice = tensor.as_slice::<f32>().unwrap();

        // First value in red channel should be 1.0 (255/255)
        assert_eq!(slice[0], 1.0);

        // First value in green channel should be 0.0
        assert_eq!(slice[640 * 640], 0.0);

        // First value in blue channel should be 0.0
        assert_eq!(slice[2 * 640 * 640], 0.0);
    }

    #[test]
    fn test_image_to_tensor_rectangle() {
        // Create a 200x100 red image
        let mut img = ImageBuffer::new(200, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([255, 0, 0]);
        }
        let image = DynamicImage::ImageRgb8(img);

        let tensor = resize_image_to_tensor(&image, 640, 640).unwrap();
        let shape = tensor.shape();
        assert_eq!(shape, &[1, 3, 640, 640]);

        // Check that the image was properly centered
        let slice = tensor.as_slice::<f32>().unwrap();
        let center_x = 320;
        let center_y = 320;

        // Calculate index for red channel (c=0) at center pixel
        let index = 0 * (640 * 640) + center_y * 640 + center_x;
        assert_eq!(slice[index], 1.0); // Red channel at center
    }

    #[test]
    fn test_image_to_tensor_normalization() {
        // Create a 100x100 gray image (128, 128, 128)
        let mut img = ImageBuffer::new(100, 100);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([128, 128, 128]);
        }
        let image = DynamicImage::ImageRgb8(img);

        let tensor = resize_image_to_tensor(&image, 640, 640).unwrap();
        let slice = tensor.as_slice::<f32>().unwrap();

        // Check that values are properly normalized to [0,1]
        let expected = 128.0 / 255.0;
        assert!((slice[0] - expected).abs() < 0.0001);
        assert!((slice[1] - expected).abs() < 0.0001);
        assert!((slice[2] - expected).abs() < 0.0001);
    }
}
