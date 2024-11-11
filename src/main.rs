use std::{mem, sync::{Arc, Mutex}};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use macroquad::prelude::*;

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


    loop {
        let mut audio_data_unwrap = audio_data.lock().unwrap();
        let audio_data_copy = audio_data_unwrap.to_vec();
        audio_data_unwrap.clear();
        mem::drop(audio_data_unwrap);
        let mut it_size = audio_data_copy.len() / screen_width() as usize;
        let it_x_delta = screen_width() / audio_data_copy.len() as f32;
        if it_size < 1 {
            it_size = 1;
        }
        let max_height: f32 = screen_height() /2.;
        for (it, data) in audio_data_copy.iter().enumerate() {
            if it % it_size != 0 {
                continue;
            }
            if it >= audio_data_copy.len() - 1 - it_size {
                break;
            }

            let next = audio_data_copy[it+it_size];
            let data_normalized = *data / 0.4;
            let delta_color: Color = Color::new(data_normalized.abs(), 0., 1. - data_normalized.abs(), 1.);
            draw_line(it as f32 * it_x_delta, (screen_height() / 2.) + (data) * max_height, (it as f32 + it_size as f32) * it_x_delta, (screen_height() / 2.) + (next) * max_height, 2., delta_color);
        }

        next_frame().await
    }
}
