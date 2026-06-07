#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceSource {
    Manual,
    CustomerFixedPrice,
    DefaultPrice,
    Zero,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PriceChoice {
    pub unit_price: f64,
    pub source: PriceSource,
}

pub fn choose_unit_price(
    manual_price: Option<f64>,
    fixed_price: Option<f64>,
    default_price: f64,
) -> PriceChoice {
    if let Some(unit_price) = manual_price {
        return PriceChoice {
            unit_price,
            source: PriceSource::Manual,
        };
    }
    if let Some(unit_price) = fixed_price {
        return PriceChoice {
            unit_price,
            source: PriceSource::CustomerFixedPrice,
        };
    }
    if default_price > 0.0 {
        return PriceChoice {
            unit_price: default_price,
            source: PriceSource::DefaultPrice,
        };
    }
    PriceChoice {
        unit_price: 0.0,
        source: PriceSource::Zero,
    }
}

pub fn threshold_times(quantity: f64, threshold_quantity: Option<f64>) -> f64 {
    threshold_quantity
        .filter(|threshold| *threshold > 0.0)
        .map(|threshold| (quantity / threshold).floor())
        .filter(|times| *times > 0.0)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_source_prefers_manual_then_customer_fixed_then_default() {
        assert_eq!(
            choose_unit_price(Some(8.0), Some(9.0), 10.0),
            PriceChoice {
                unit_price: 8.0,
                source: PriceSource::Manual
            }
        );
        assert_eq!(
            choose_unit_price(None, Some(9.0), 10.0),
            PriceChoice {
                unit_price: 9.0,
                source: PriceSource::CustomerFixedPrice
            }
        );
        assert_eq!(
            choose_unit_price(None, None, 10.0),
            PriceChoice {
                unit_price: 10.0,
                source: PriceSource::DefaultPrice
            }
        );
        assert_eq!(
            choose_unit_price(None, None, 0.0),
            PriceChoice {
                unit_price: 0.0,
                source: PriceSource::Zero
            }
        );
    }

    #[test]
    fn threshold_times_uses_full_multiples_only() {
        assert_eq!(threshold_times(9.0, Some(5.0)), 1.0);
        assert_eq!(threshold_times(10.0, Some(5.0)), 2.0);
        assert_eq!(threshold_times(4.0, Some(5.0)), 0.0);
        assert_eq!(threshold_times(10.0, Some(0.0)), 0.0);
        assert_eq!(threshold_times(10.0, None), 0.0);
    }
}
