use std::{fs::File, io::BufWriter};

use image::{self, GenericImageView};

use realfft::RealFftPlanner;

use byteorder::WriteBytesExt;

fn main() {
    let mut args = std::env::args();
    
    let input = args.nth(1).expect("No input file");
    let output = args.nth(0).expect("No output file");

    let image = image::open(input)
        .unwrap()
        .to_luma32f();

    let fft = RealFftPlanner::<f32>::new()
        .plan_fft_inverse(1024);

    // All sound libraries in Rust are pussy. Use raw 32-bit floating point PCM.
    let mut sound = BufWriter::new(File::create(output).unwrap());

    let mut fft_input = fft.make_input_vec();
    let mut fft_scratch = fft.make_scratch_vec();
    let mut fft_output = fft.make_output_vec();
    let fft_len = fft_input.len() as f32;

    let image_bounds = image.bounds();
    let image_height = image.height() as f32;

    for j in image_bounds.0..image_bounds.2
    {
        for (i, s) in fft_input.iter_mut()
            .rev()
            .enumerate() 
        {
            s.re = image[(
                j as u32,
                // Translate point in fft input buffer to output.
                // Nearest neighbor interpolation.
                (i as f32 / fft_len * image_height) as u32 + image_bounds.1
            )].0[0] / 255.0;

            // For 0 deg phase shift sin(0) = 0.
            s.im = 0.0;
        }

        fft.process_with_scratch(
            fft_input.as_mut_slice(), 
            fft_output.as_mut_slice(), 
            fft_scratch.as_mut_slice()
        ).unwrap();

        for s in fft_output.iter() {
            sound.write_f32::<byteorder::LE>(*s).unwrap();
        }
    }
}