// Command quant demonstrates the Guardrail quant endpoints via the Go SDK.
//
// It targets a local API (GUARDRAIL_API or http://localhost:8080), calls each
// quant endpoint read-only with a short deadline, and prints results. It exits
// 0 even when the API is unreachable so it is safe to run offline.
package main

import (
	"context"
	"fmt"
	"os"
	"time"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

func main() {
	base := os.Getenv("GUARDRAIL_API")
	if base == "" {
		base = "http://localhost:8080"
	}
	client := guardrail.NewClient(base, guardrail.WithTimeout(5*time.Second))
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	show := func(name string, v map[string]any, err error) {
		if err != nil {
			fmt.Printf("%-12s error (is the API running?): %v\n", name, err)
			return
		}
		fmt.Printf("%-12s %v\n", name, v)
	}

	ta, err := client.TA(ctx, "rsi", []float64{44, 44.3, 44.1, 43.6, 44.3, 44.8, 45.1, 45.4, 45.8, 46.0}, 5, 0)
	show("ta", ta, err)

	fees, err := client.Fees(ctx, map[string]string{"notional_usd": "25000", "quantity": "12", "side": "buy"})
	show("fees", fees, err)

	size, err := client.Sizer(ctx, "kelly", map[string]string{"win_prob": "0.6", "odds": "1.5"})
	show("sizer", size, err)

	pnl, err := client.PnL(ctx, "CAKE,buy,10,2;CAKE,sell,4,3", "CAKE:3")
	show("pnl", pnl, err)
}
