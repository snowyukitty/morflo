#[derive(Debug, Clone)]
pub struct ProgressParser {
    duration_seconds: Option<f64>,
    pending_seconds: Option<f64>,
    last_fraction: f64,
}

impl ProgressParser {
    pub fn new(duration_seconds: Option<f64>) -> Self {
        Self {
            duration_seconds: duration_seconds.filter(|value| value.is_finite() && *value > 0.0),
            pending_seconds: None,
            last_fraction: 0.0,
        }
    }

    pub fn push_line(&mut self, line: &str) -> Option<f64> {
        let (key, value) = line.trim().split_once('=')?;
        match key {
            "out_time_us" | "out_time_ms" => {
                self.pending_seconds = value.parse::<f64>().ok().map(|value| value / 1_000_000.0);
                None
            }
            "out_time" => {
                self.pending_seconds = parse_clock(value);
                None
            }
            "progress" if value == "end" => {
                self.last_fraction = 1.0;
                Some(1.0)
            }
            "progress" => self.fraction(),
            _ => None,
        }
    }

    fn fraction(&mut self) -> Option<f64> {
        let duration = self.duration_seconds?;
        let seconds = self.pending_seconds?;
        let fraction = (seconds / duration).clamp(self.last_fraction, 0.995);
        self.last_fraction = fraction;
        Some(fraction)
    }
}

fn parse_clock(value: &str) -> Option<f64> {
    let mut fields = value.split(':');
    let hours = fields.next()?.parse::<f64>().ok()?;
    let minutes = fields.next()?.parse::<f64>().ok()?;
    let seconds = fields.next()?.parse::<f64>().ok()?;
    (fields.next().is_none() && minutes < 60.0 && seconds < 60.0)
        .then_some(hours * 3_600.0 + minutes * 60.0 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_truthful_ffmpeg_progress() {
        let mut parser = ProgressParser::new(Some(10.0));
        assert_eq!(parser.push_line("out_time_us=2500000"), None);
        assert_eq!(parser.push_line("progress=continue"), Some(0.25));
        assert_eq!(parser.push_line("progress=end"), Some(1.0));
    }

    #[test]
    fn remains_indeterminate_without_duration() {
        let mut parser = ProgressParser::new(None);
        assert_eq!(parser.push_line("out_time=00:00:02.500000"), None);
        assert_eq!(parser.push_line("progress=continue"), None);
        assert_eq!(parser.push_line("progress=end"), Some(1.0));
    }

    #[test]
    fn never_moves_backwards_or_claims_completion_early() {
        let mut parser = ProgressParser::new(Some(10.0));
        parser.push_line("out_time_us=9000000");
        assert_eq!(parser.push_line("progress=continue"), Some(0.9));
        parser.push_line("out_time_us=3000000");
        assert_eq!(parser.push_line("progress=continue"), Some(0.9));
        parser.push_line("out_time_us=12000000");
        assert_eq!(parser.push_line("progress=continue"), Some(0.995));
    }
}
