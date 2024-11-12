use std::{mem, sync::{Arc, Mutex}};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use macroquad::prelude::*;
use rustfft::{num_complex::{Complex, ComplexFloat}, num_traits::{ConstZero, Float, Zero}, FftPlanner};

#[macroquad::main("Music visualizer")]
async fn main() {
    let audio_host = cpal::default_host();

    let device = audio_host.default_output_device().expect("Failed to get default output device");

    let cfg = device.default_output_config().expect("Failed to get default config");

    let audio_data = Arc::new(Mutex::new(Vec::new()));

    let audio_data_clone = Arc::clone(&audio_data);
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_input_stream(
                &cfg.into(), 
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut audio_data = audio_data_clone.lock().unwrap();
                    audio_data.extend_from_slice(data);
                },
                move |err| eprintln!("Stream error: {}", err),
                None
            )
        },
        _ => panic!("Unsupported sample format"),

    }.expect("Failed to create input stream");

    stream.play().expect("Failed to play stream");
    const SAMPLES: usize = 4800;

    let mut fft_planner = FftPlanner::<f32>::new();
    let fft = fft_planner.plan_fft_forward(SAMPLES);

    let mut buffer: Vec<Complex<f32>> = vec![Complex::zero(); SAMPLES];

    let num_bins = 1024; // Number of visual bins
    let min_freq = 20.; // Minimum frequency (Hz)
    let max_freq = 20000.0; // Maximum frequency (Hz)
    let sample_rate = 48000.;
    let nyquist = sample_rate / 2.0;
    let mut magnitudes_hr: Vec<f32> = vec![0.; SAMPLES/2];

    loop {
        let mut audio_data_unwrap = audio_data.lock().unwrap();
        let audio_data_copy = audio_data_unwrap.to_vec();
        render_frequency(&mut magnitudes_hr);
        if audio_data_copy.len() >= SAMPLES {
            audio_data_unwrap.drain(0..SAMPLES);
            mem::drop(audio_data_unwrap);
            for i in 0..SAMPLES{
                buffer[i] = Complex::new(audio_data_copy[i], 0.);
            }

            fft.process(&mut buffer);
            
            let mut magnitudes: Vec<f32> = buffer.iter()
                .take(SAMPLES/2)
                .map(|c| c.norm()).collect();


            let mut hr_it = 0;
            for (i, &data) in magnitudes.iter().enumerate() {
                let freq = (i as f32 * nyquist) / SAMPLES as f32 / 2.;
                if freq < min_freq || freq > max_freq {
                    continue;
                }
                
                let mag_scaled = data / SAMPLES as f32 / 2.;
                let mag_scaled_db = 20. * mag_scaled.log10();
                magnitudes_hr[hr_it] = mag_scaled_db;
                hr_it += 1;
            }

            //render_audio_wave(audio_data_copy);
        } else {
            mem::drop(audio_data_unwrap);
        }
        
        //render_audio_wave(audio_data_copy);

        next_frame().await
    }
}

fn render_frequency(magnitudes: &mut Vec<f32>) {
    //magnitudes.retain(|&f| f >= 1.);
    let it_x_delta = screen_width() / magnitudes.len() as f32;
    let mut custom_it = 0;
    for (it, &data) in magnitudes.iter().enumerate() {
        if it >= magnitudes.len() - 2 {
            break;
        }
        draw_line(custom_it as f32 * it_x_delta, screen_height(), custom_it as f32 * it_x_delta, screen_height()/2. - data, it_x_delta, WHITE);
        custom_it += 1;
    }
}

fn render_audio_wave(audio_data: Vec<f32>) {
    let mut it_size = audio_data.len() / screen_width() as usize;
    let it_x_delta = screen_width() / audio_data.len() as f32;
    if it_size < 1 {
        it_size = 1;
    }
    let max_height: f32 = screen_height() /4.;
    for (it, data) in audio_data.iter().enumerate() {
        if it % it_size != 0 {
            continue;
        }
        if it >= audio_data.len() - 1 - it_size {
            break;
        }

        let next = audio_data[it+it_size];
        let data_normalized = *data / 0.4;
        let delta_color: Color = Color::new(data_normalized.abs(), 0., 1. - data_normalized.abs(), 1.);
        draw_line(it as f32 * it_x_delta, (screen_height() / 2.) + (data) * max_height, (it as f32 + it_size as f32) * it_x_delta, (screen_height() / 2.) + (next) * max_height, 2., delta_color);
    }
}
