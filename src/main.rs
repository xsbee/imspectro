use std::{fs::File, io::BufWriter};

use image::{self, GenericImageView};

use realfft::RealFftPlanner;

use byteorder::WriteBytesExt;

use std::f32::consts::PI;

fn hann_window(lut: &mut [f32]) {
    #![allow(non_snake_case)]
    let N = lut.len();

    for n in 0..N {
        let y = PI * n as f32 / N as f32;
        lut[n] = (y * y).sin();
    }
}

fn main() {
    let mut args = std::env::args();

    let input = args.nth(1).expect("No input file");
    let output = args.nth(0).expect("No output file");
    let fft_len = args
        .nth(0)
        .map(|x| x.parse::<usize>().ok())
        .flatten()
        .unwrap_or(2048);

    let image = image::open(input).unwrap().to_luma32f();

    let ifft = RealFftPlanner::<f32>::new().plan_fft_inverse(fft_len);

    // All sound libraries in Rust are pussy. Use raw 32-bit floating point PCM.
    let mut sound = BufWriter::new(File::create(output).unwrap());

    let mut ifft_input = ifft.make_input_vec();
    let mut ifft_scratch = ifft.make_scratch_vec();
    let mut ifft_output = ifft.make_output_vec();
    let mut hann = ifft.make_output_vec();
    let mut sliding = vec![0f32; fft_len];
    let ifft_len = ifft_input.len() as f32;

    hann_window(&mut hann);

    let image_bounds = image.bounds();
    let image_height = image.height() as f32;

    for j in image_bounds.0..image_bounds.2 {
        let mut sum = 0.0;

        for (i, s) in ifft_input[1..].iter_mut().rev().enumerate() {
            s.re = image[(
                j as u32,
                // Translate point in ifft input buffer to output.
                // Nearest neighbor interpolation.
                (i as f32 / (ifft_len - 1.0) * image_height) as u32 + image_bounds.1,
            )]
                .0[0]
                / 255.0;

            sum += s.re;

            // For 0 deg phase shift sin(0) = 0.
            s.im = 0.0;
        }

        ifft_input[0].re = sum / ifft_len;
        ifft_input[0].im = 0.0;

        ifft.process_with_scratch(
            ifft_input.as_mut_slice(),
            ifft_output.as_mut_slice(),
            ifft_scratch.as_mut_slice(),
        )
        .unwrap();

        #[allow(non_snake_case)]
        let Nhalf = sliding.len();

        // TODO fix code duplication
        sliding.drain(..Nhalf);
        sliding.extend(ifft_output[..Nhalf].iter());

        for (i, s) in sliding.iter_mut().enumerate() {
            *s *= hann[i];
        }

        for s in sliding.iter() {
            sound.write_f32::<byteorder::LE>(*s).unwrap();
        }

        sliding.drain(..Nhalf);
        sliding.extend(ifft_output[Nhalf..].iter());

        for (i, s) in sliding.iter_mut().enumerate() {
            *s *= hann[i];
        }

        for s in sliding.iter() {
            sound.write_f32::<byteorder::LE>(*s).unwrap();
        }
    }
}
