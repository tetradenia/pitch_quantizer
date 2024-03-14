pub fn amplitude_from_complex(re: f32, im: f32) -> f32 {
    (re.powi(2) + im.powi(2)).sqrt()
}

pub fn bucket_to_freq(n: i32, sample_rate: f32, window_size: usize) -> f32 {
    n as f32 * sample_rate / window_size as f32
}

pub fn bucket_spread(center: i32, center_proportion: f32, max_bucket: i32, spread_falloff: f32) -> Vec<f32> {
    let mut to_return: Vec<f32> = (0..max_bucket).map(|_| {0f32}).collect();
    to_return[center as usize] = center_proportion;
    let mut drop_amount = (1f32 - center_proportion) * spread_falloff;
    let mut offset = 1;
    while drop_amount > 0.0001 {
        if (center - offset) >= 0 {
            to_return[(center-offset) as usize] = drop_amount;
        }

        if (center + offset) < max_bucket {
            to_return[(center+offset) as usize] = drop_amount;
        }

        drop_amount *= spread_falloff;
        offset += 1;
    }
    to_return
}

pub fn closest_bucket_to_freq(freq: f32, sample_rate: f32, window_size: usize) -> i32 {
    let freq_per_bin = sample_rate / window_size as f32;
    (freq / freq_per_bin).round() as i32
}

pub fn lazy_upward_round(freq: f32, round_to: &Vec<f32>) -> f32 {
    for threshold in round_to {
        if freq < *threshold { return *threshold }
    }
    0f32
}
