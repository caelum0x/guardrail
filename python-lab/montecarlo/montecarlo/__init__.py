"""Monte Carlo simulation of strategy equity outcomes (bootstrap + GBM)."""

from .sim import SimResult, bootstrap, estimate_mu_sigma, gbm, returns_from_equity

__all__ = ["SimResult", "bootstrap", "gbm", "estimate_mu_sigma", "returns_from_equity"]
