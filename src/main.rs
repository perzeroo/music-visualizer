use std::{mem, sync::{Arc, Mutex}};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use macroquad::prelude::*;
use rustfft::{num_complex::{Complex, ComplexFloat}, num_traits::{Float, Zero}, FftPlanner};


static mut MAX_EVER_VAL: f32 = 0.0;
#[macroquad::main("Music visualizer")]
async fn main() {
    let audio_host = cpal::default_host();

    let device = audio_host.default_output_device().expect("Failed to get default output device");

    let cfg = device.default_output_config().expect("Failed to get default config");

    // How many samples we use for visualization, this should be about the 10th or 20th of the
    // sample rate. Must be set here as it is moved at line 27.
    let samples: usize = cfg.sample_rate().0 as usize / 10;
    let sample_rate = cfg.sample_rate().0 as f32;

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

    let mut fft_planner = FftPlanner::<f32>::new();
    let fft = fft_planner.plan_fft_forward(samples);

    let mut buffer: Vec<Complex<f32>> = vec![Complex::zero(); samples];

    let min_freq = 20.; // Minimum frequency (Hz)
    let max_freq = 20000.0; // Maximum frequency (Hz)
    let nyquist = sample_rate / 2.0;
    // needs a better way of getting the size, this probably wont work with anything other than
    // 48 000Hz sample rate, its required for rendering so we arent rendering the frequencies we
    // cant hear
    let mut new_magnitudes_hr: Vec<f32> = vec![0.; samples/4 - 199];
    let mut magnitudes_hr: Vec<f32> = vec![0.; samples/4 - 199];
    // Over how many samples do we average, this smooths out the visualization so the peaks aren't
    // as intense
    let avg_factor = 16;

    // Render loop
    loop {
        let mut audio_data_unwrap = audio_data.lock().unwrap();
        let audio_data_copy = audio_data_unwrap.to_vec();
        clear_background(BLACK);
        render_frequency(&mut magnitudes_hr);
        if audio_data_copy.len() >= samples {
            audio_data_unwrap.drain(0..samples);
            mem::drop(audio_data_unwrap);
            for i in 0..samples{
                buffer[i] = Complex::new(audio_data_copy[i], 0.);
            }

            fft.process(&mut buffer);
            
            // We take half of the samples as the rest is a mirrored copy, then normalize them
            let magnitudes: Vec<f32> = buffer.iter()
                .take(samples/2)
                .map(|c| c.norm()).collect();

            // Reduce the magnitudes to hearing range (hr = hearing range). Smooth out the peaks,
            // convert to dB
            let mut hr_it = 0;
            for (i, &data) in magnitudes.iter().enumerate() {
                let freq = (i as f32 * nyquist) / (samples as f32 / 4.);
                if freq < min_freq || freq > max_freq {
                    continue;
                }
                
                //let freq_scale = (freq / max_freq).exp();
                let freq_scale = 2595. * (1. + freq.max(1000.)/700.).log10();
                let mag_scaled_db = ((20. * data.log10()).max(0.) * freq_scale) * avg_factor as f32;
                let mut total_mag = mag_scaled_db;
                let mut total_count = avg_factor;
                for k in 0..avg_factor {
                    let l_freq = ((i+k) as f32 * nyquist) / (samples as f32 / 4.);

                    let l_freq_scale = 2595. * (1. + (l_freq).max(1000.) /700.).log10();
                    total_mag += ((20. * magnitudes[i+k].log10()).max(0.) * l_freq_scale) * (avg_factor as f32 - k as f32);
                    total_count += avg_factor - k;
                }
                let avg_mag = total_mag / total_count as f32;
                new_magnitudes_hr[hr_it] = avg_mag;
                hr_it += 1;
            }
        } else {
            mem::drop(audio_data_unwrap);
        }
        if audio_data_copy.len() == 0 {
            // We fill with 0.1 so it drops to 0 when we there isnt anything playing back
            new_magnitudes_hr.fill(0.1);
        }
        // We can use either moving average or exponential smoothing, I havent found much of a
        // differnce between the two, it is certainly nicer than without any kind of smoothing
        //moving_avg(&mut magnitudes_hr, &new_magnitudes_hr, 0.25);
        exp_smoothing(&mut magnitudes_hr, &new_magnitudes_hr, 0.25);
        
        // Render the raw audio wave
        //render_audio_wave(audio_data_copy);

        next_frame().await
    }
}

fn moving_avg(samples: &mut Vec<f32>, new_samples: &Vec<f32>, alpha: f32) {
    for (i, value) in samples.iter_mut().enumerate() {
        *value = alpha * new_samples[i] + (1. - alpha) * *value; 
    }
}

fn exp_smoothing(samples: &mut Vec<f32>, new_samples: &Vec<f32>, smoothing_factor: f32) {
    for (i, value) in samples.iter_mut().enumerate() {
        *value += smoothing_factor * (new_samples[i] - *value);
    }
}

fn render_frequency(magnitudes: &mut Vec<f32>) {
    let it_x_delta = screen_width() / magnitudes.len() as f32;
    let mut current_x = 0.;
    let mut max_val = magnitudes.iter().cloned().fold(0.0_f32, f32::max);
    unsafe {
        if max_val > MAX_EVER_VAL {
            MAX_EVER_VAL = max_val;
        } else {
            max_val = MAX_EVER_VAL;
        }
    }
    for (it, &data) in magnitudes.iter().enumerate() {
        if it >= magnitudes.len() - 2 {
            break;
        }
        // stretches out the low frequencies, causes artefacts (1 pixel wide black lines)
        let x_delta = it_x_delta * ((magnitudes.len() as f32 - it as f32) / (4800. / 9.9));
        let data_normal = data / max_val;
        let percentage = it as f32 / magnitudes.len() as f32;
        let color: Color = Color::new(1. - percentage.powf(0.5), 0., percentage.powf(0.5), 1.);
        draw_line(current_x, screen_height(), current_x, screen_height() - (data_normal * screen_height() /2.), x_delta, color);
        current_x += x_delta;
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
