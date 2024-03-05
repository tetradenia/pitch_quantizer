pub fn amplitude_from_complex(re: f32, im: f32) -> f32 {
    (re.powi(2) + im.powi(2)).sqrt()
}

pub fn bucket_to_freq(n: i32, sample_rate: f32, window_size: usize) -> f32 {
    n as f32 * sample_rate / window_size as f32
}

pub fn closest_bucket_to_freq(freq: f32, sample_rate: f32, window_size: usize) -> i32 {
    let freq_per_bin = sample_rate / window_size as f32;
    (freq / freq_per_bin).round() as i32
}

pub fn lazy_upward_round(freq: f32, round_to: &[f32]) -> f32 {
    for threshold in round_to {
        if freq < *threshold { return *threshold }
    }
    0f32
}
