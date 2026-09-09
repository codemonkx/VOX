use std::f32::consts::PI;
use rustfft::{FftPlanner, num_complex::Complex};

pub struct SpectrumAnalyzer {
    planner: FftPlanner<f32>,
    fft_size: usize,
    window: Vec<f32>,
    prev_levels: Vec<f32>,
    num_bars: usize,
}

impl SpectrumAnalyzer {
    pub fn new(num_bars: usize) -> Self {
        let fft_size = 1024;
        let window: Vec<f32> = (0..fft_size)
            .map(|n| 0.5 * (1.0 - (2.0 * PI * n as f32 / (fft_size - 1) as f32).cos()))
            .collect();
        Self {
            planner: FftPlanner::new(),
            fft_size,
            window,
            prev_levels: vec![0.0; num_bars],
            num_bars,
        }
    }

    /// Computes bar levels in range 0..=8 for the visualizer.
    pub fn compute_bars(&mut self, samples: &[f32], sample_rate: u32, is_playing: bool) -> Vec<usize> {
        let decay_rate = 0.40; // Cava-like gravitational falloff per frame
        if !is_playing || samples.len() < 64 {
            for level in &mut self.prev_levels {
                *level = (*level - decay_rate).max(0.0);
            }
            return self.prev_levels.iter().map(|&l| (l.round() as usize).min(8)).collect();
        }

        let sr = if sample_rate == 0 { 44100 } else { sample_rate } as f32;
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .take(self.fft_size)
            .zip(self.window.iter())
            .map(|(&s, &w)| Complex { re: s * w, im: 0.0 })
            .collect();

        while buffer.len() < self.fft_size {
            buffer.push(Complex { re: 0.0, im: 0.0 });
        }

        let fft = self.planner.plan_fft_forward(self.fft_size);
        fft.process(&mut buffer);

        // Magnitude calculation for positive frequencies (first fft_size / 2 bins)
        let num_bins = self.fft_size / 2;
        let mut magnitudes = Vec::with_capacity(num_bins);
        for item in buffer.iter().take(num_bins) {
            let mag = (item.re * item.re + item.im * item.im).sqrt()
                / (self.fft_size as f32 / 2.0);
            magnitudes.push(mag);
        }

        // Define logarithmic frequency cutoff ranges
        let min_freq = 35.0f32;
        let max_freq = (sr / 2.0).min(16000.0);
        let bin_hz = sr / (self.fft_size as f32);

        let mut raw_bars = vec![0.0f32; self.num_bars];

        for (i, bar) in raw_bars.iter_mut().enumerate().take(self.num_bars) {
            let f_low = min_freq * (max_freq / min_freq).powf(i as f32 / self.num_bars as f32);
            let f_high = min_freq * (max_freq / min_freq).powf((i + 1) as f32 / self.num_bars as f32);

            let k_low = ((f_low / bin_hz).floor() as usize).min(num_bins - 1);
            let k_high = ((f_high / bin_hz).ceil() as usize).max(k_low + 1).min(num_bins);

            let mut sum = 0.0f32;
            let mut count = 0;
            for &mag in magnitudes.iter().take(k_high).skip(k_low) {
                sum += mag;
                count += 1;
            }
            let avg = if count > 0 { sum / count as f32 } else { 0.0 };

            // Frequency weighting (equal loudness compensation):
            // Bass inherently has more power in mixdowns, so boost upper mids/treble
            let freq_weight = 1.0 + (i as f32 / self.num_bars as f32) * 2.2;
            let weighted = avg * freq_weight;

            // Perceptual scaling: square root compression mapped to 0..8
            let scaled = (weighted * 35.0).sqrt() * 4.8;
            *bar = scaled.clamp(0.0, 8.0);
        }

        // Cava-style fast attack and smooth gravitational decay
        for (i, &target) in raw_bars.iter().enumerate().take(self.num_bars) {
            if target >= self.prev_levels[i] {
                // Instant attack
                self.prev_levels[i] = target;
            } else {
                // Smooth decay
                self.prev_levels[i] = (self.prev_levels[i] - decay_rate).max(target);
            }
        }

        self.prev_levels.iter().map(|&l| (l.round() as usize).min(8)).collect()
    }
}
