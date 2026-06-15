// Command verify independently re-derives and checks a Guardrail BNB AI-Agent
// proof, then prints a PASS/FAIL report.
//
// It loads a proof from a file argument (defaulting to the bundled
// clients/proof-verifier/sample_proof.json fixture) or, with -url, fetches the
// live /proof envelope from a running API. Either way it verifies the proof
// fully offline using only the standard library and exits 0 even when the file
// or API is unavailable, so it is safe to run in CI or against an offline
// backend.
//
// Usage:
//
//	go run ./example/verify [PROOF_JSON]
//	go run ./example/verify -url http://localhost:8080
//	go run ./example/verify -policy-file ../../configs/risk_policy.paper.json sample_proof.json
package main

import (
	"context"
	"flag"
	"log"
	"time"

	guardrail "github.com/guardrail-alpha/guardrail-go"
)

// defaultProofCandidates are tried in order when no proof path is supplied, so
// the example finds the bundled fixture whether it is run from clients/go
// (go run ./example/verify) or from the command's own directory.
var defaultProofCandidates = []string{
	"../../proof-verifier/sample_proof.json", // CWD = clients/go/example/verify
	"../proof-verifier/sample_proof.json",    // CWD = clients/go
	"clients/proof-verifier/sample_proof.json",
}

func main() {
	log.SetFlags(0)

	url := flag.String("url", "", "fetch /proof from this API base URL instead of a file")
	policyFile := flag.String("policy-file", "", "policy file to recompute policy_hash against (optional)")
	timeout := flag.Duration("timeout", 5*time.Second, "request timeout when using -url")
	flag.Parse()

	proof, source, err := loadProof(*url, flag.Arg(0), *timeout)
	if err != nil {
		// Graceful: the proof source was unavailable. Report and exit 0 so the
		// example is safe to run offline.
		log.Printf("proof source unavailable: %v", err)
		log.Println("nothing to verify; exiting cleanly (0)")
		return
	}

	result := proof.Verify(source, *policyFile)
	log.Println(result.Report())
}

// loadProof resolves the proof from -url, an explicit file argument, or the
// default fixture, returning the parsed proof and a source label for reporting.
func loadProof(url, arg string, timeout time.Duration) (*guardrail.Proof, string, error) {
	if url != "" {
		client := guardrail.NewClient(url, guardrail.WithTimeout(timeout))
		ctx, cancel := context.WithTimeout(context.Background(), timeout)
		defer cancel()
		proof, err := client.Proof(ctx)
		if err != nil {
			return nil, url + "/proof", err
		}
		return proof, url + "/proof", nil
	}

	if arg != "" {
		proof, err := guardrail.LoadProofFile(arg)
		if err != nil {
			return nil, arg, err
		}
		return proof, arg, nil
	}

	// No explicit path: try the bundled-fixture candidates in order.
	var firstErr error
	for _, path := range defaultProofCandidates {
		proof, err := guardrail.LoadProofFile(path)
		if err == nil {
			return proof, path, nil
		}
		if firstErr == nil {
			firstErr = err
		}
	}
	return nil, defaultProofCandidates[0], firstErr
}
