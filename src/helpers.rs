use realfft::num_traits::Float;

pub fn amplitude_from_complex(re: f32, im: f32) -> f32 {
    (re.powi(2) + im.powi(2)).sqrt()
}

pub fn bucket_to_freq(n: i32, sample_rate: i32, window_size: i32) -> f32 {
    n as f32 * sample_rate as f32 / window_size as f32
}

pub fn closest_bucket_to_freq(freq: f32, sample_rate: i32, window_size: i32) -> i32 {
    let freq_per_bin = sample_rate as f32 / window_size as f32;
    (freq / freq_per_bin).round() as i32
}
