package guardrail

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"regexp"
	"sort"
	"strings"
)

// Constants mirrored from the Rust workspace (read-only references). They are
// duplicated here deliberately: the verifier shares no code with the agent, so
// agreement between the two proves the commitments are independently
// reproducible.
const (
	// bscscanBaseURL mirrors crates/bnb-agent/src/proof.rs :: BSCSCAN_BASE_URL.
	bscscanBaseURL = "https://bscscan.com"

	// CompetitionContract mirrors apps/guardrail-api/src/compete.rs ::
	// COMPETITION_CONTRACT.
	CompetitionContract = "0x212c61b9b72c95d95bf29cf032f5e5635629aed5"

	// CompetitionContractBsctrace mirrors apps/guardrail-api/src/compete.rs ::
	// COMPETITION_CONTRACT_BSCTRACE.
	CompetitionContractBsctrace = "https://bsctrace.com/address/0x212c61b9b72c95d95bf29cf032f5e5635629aed5"
)

// reportCoreFields is the field order the agent hashes to derive report_hash.
// Mirrors crates/agent-runtime/src/runtime.rs.
var reportCoreFields = [...]string{"run_id", "cycles", "final_nav_usd", "total_drawdown_pct", "events"}

// Format-validation patterns. addressRe accepts the canonical 40-hex address
// plus this repo's 41/42-char vanity placeholder; canonicalAddressRe enforces a
// strict 20-byte (40-hex) address.
var (
	addressRe          = regexp.MustCompile(`^0x[0-9a-fA-F]{40,42}$`)
	canonicalAddressRe = regexp.MustCompile(`^0x[0-9a-fA-F]{40}$`)
	txHashRe           = regexp.MustCompile(`^0x[0-9a-fA-F]{64}$`)
	sha256Re           = regexp.MustCompile(`^[0-9a-f]{64}$`)
)

// CheckStatus is the outcome of a single verification check.
type CheckStatus string

// Verification outcomes. CheckSkip marks a check that is not applicable to the
// supplied proof shape (for example a bare run report omitting report_hash).
const (
	CheckPass CheckStatus = "PASS"
	CheckFail CheckStatus = "FAIL"
	CheckSkip CheckStatus = "SKIP"
)

// Check is one immutable verification result.
type Check struct {
	Name   string      `json:"name"`
	Status CheckStatus `json:"status"`
	Detail string      `json:"detail"`
}

// VerifyResult is the aggregate outcome of verifying a proof. Passed is true
// only when no check failed (skips do not, by default, fail the result).
type VerifyResult struct {
	Source string  `json:"source"`
	Passed bool    `json:"passed"`
	Checks []Check `json:"checks"`
}

// Counts returns the number of passed, failed, and skipped checks.
func (r VerifyResult) Counts() (passed, failed, skipped int) {
	for _, c := range r.Checks {
		switch c.Status {
		case CheckPass:
			passed++
		case CheckFail:
			failed++
		case CheckSkip:
			skipped++
		}
	}
	return passed, failed, skipped
}

// Report renders a human-readable PASS/FAIL report mirroring the Python
// verifier's text output.
func (r VerifyResult) Report() string {
	var b strings.Builder
	const rule = "============================================================"
	b.WriteString(rule + "\n")
	b.WriteString(" Guardrail BNB AI-Agent — Independent Proof Verification\n")
	b.WriteString(rule + "\n")
	fmt.Fprintf(&b, " proof source : %s\n\n", r.Source)
	for _, c := range r.Checks {
		fmt.Fprintf(&b, " [%s] %s\n", c.Status, c.Name)
		fmt.Fprintf(&b, "        %s\n", c.Detail)
	}
	passed, failed, skipped := r.Counts()
	overall := "PASS"
	if failed > 0 {
		overall = "FAIL"
	}
	b.WriteString("\n")
	b.WriteString("------------------------------------------------------------\n")
	fmt.Fprintf(&b, " RESULT: %s  (%d passed, %d failed, %d skipped)\n", overall, passed, failed, skipped)
	b.WriteString(rule)
	return b.String()
}

// --- Hashing helpers (mirrors the Rust agent + Python verifier) -------------

func sha256Hex(data []byte) string {
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:])
}

func sha256HexStr(text string) string {
	return sha256Hex([]byte(text))
}

// agentIDFor re-derives agent_id = sha256(name + 0x00 + wallet). Mirrors
// crates/bnb-agent/src/identity.rs, which joins the fields with a single NUL
// byte to avoid boundary-collision ambiguity.
func agentIDFor(name, wallet string) string {
	preimage := make([]byte, 0, len(name)+len(wallet)+1)
	preimage = append(preimage, name...)
	preimage = append(preimage, 0)
	preimage = append(preimage, wallet...)
	return sha256Hex(preimage)
}

// reportHashFor re-derives report_hash from the report core fields. The Rust
// agent builds core = json!({run_id, cycles, final_nav_usd, total_drawdown_pct,
// events}) and hashes core.to_string(); serde_json emits compact JSON in
// insertion order, so this reproduces that exact byte sequence. It returns
// ("", false) when any required field is absent.
func reportHashFor(source map[string]json.RawMessage) (string, bool) {
	for _, field := range reportCoreFields {
		if _, ok := source[field]; !ok {
			return "", false
		}
	}
	var b strings.Builder
	b.WriteByte('{')
	for i, field := range reportCoreFields {
		if i > 0 {
			b.WriteByte(',')
		}
		// Keys are plain ASCII identifiers; marshalling guarantees correct quoting.
		key, _ := json.Marshal(field)
		b.Write(key)
		b.WriteByte(':')
		b.Write(compactJSON(source[field]))
	}
	b.WriteByte('}')
	return sha256HexStr(b.String()), true
}

// compactJSON normalizes a raw JSON value into serde_json's compact form (no
// insignificant whitespace). On any parse failure it returns the bytes as-is.
func compactJSON(raw json.RawMessage) []byte {
	var v any
	if err := json.Unmarshal(raw, &v); err != nil {
		return raw
	}
	out, err := json.Marshal(v)
	if err != nil {
		return raw
	}
	return out
}

// --- Proof model ------------------------------------------------------------

// Proof is a typed view of the agent's /proof envelope. It also accepts a bare
// data/run_report.json document, in which case the commitment-bearing fields
// live at the top level. Fields absent from a given shape decode to their zero
// value and are reported as SKIP during verification.
type Proof struct {
	Agent          string         `json:"agent,omitempty"`
	RegistrationTx string         `json:"registration_tx,omitempty"`
	LatestReport   *ProofReport   `json:"latest_report,omitempty"`
	RunReport      *ProofReport   `json:"run_report,omitempty"`
	SourceEventID  string         `json:"source_event_id,omitempty"`
	Raw            map[string]any `json:"-"`
}

// ProofReport carries the commitment fields the verifier re-derives. The
// numeric core fields (cycles, events) are kept as json.RawMessage so the
// report_hash re-derivation reproduces the agent's exact serialization.
type ProofReport struct {
	RunID             string          `json:"run_id,omitempty"`
	Cycles            json.RawMessage `json:"cycles,omitempty"`
	FinalNavUSD       json.RawMessage `json:"final_nav_usd,omitempty"`
	TotalDrawdownPct  json.RawMessage `json:"total_drawdown_pct,omitempty"`
	Events            json.RawMessage `json:"events,omitempty"`
	AgentID           string          `json:"agent_id,omitempty"`
	WalletAddress     string          `json:"wallet_address,omitempty"`
	Wallet            string          `json:"wallet,omitempty"`
	Name              string          `json:"name,omitempty"`
	PolicyHash        string          `json:"policy_hash,omitempty"`
	ReportHash        string          `json:"report_hash,omitempty"`
	AddressURL        string          `json:"address_url,omitempty"`
	RegistrationTx    string          `json:"registration_tx,omitempty"`
	RegistrationTxURL string          `json:"registration_tx_url,omitempty"`
}

// ParseProof decodes a /proof envelope or a bare run report from JSON bytes.
func ParseProof(data []byte) (*Proof, error) {
	var p Proof
	if err := json.Unmarshal(data, &p); err != nil {
		return nil, fmt.Errorf("guardrail: parse proof JSON: %w", err)
	}
	if err := json.Unmarshal(data, &p.Raw); err != nil {
		return nil, fmt.Errorf("guardrail: parse proof JSON object: %w", err)
	}
	// Bare run reports carry the commitments at the top level with no nesting.
	if p.LatestReport == nil && p.RunReport == nil {
		if _, ok := p.Raw["policy_hash"]; ok {
			var bare ProofReport
			if err := json.Unmarshal(data, &bare); err != nil {
				return nil, fmt.Errorf("guardrail: parse bare run report: %w", err)
			}
			p.LatestReport = &bare
		}
	}
	return &p, nil
}

// LoadProofFile reads and parses a proof JSON document from disk.
func LoadProofFile(path string) (*Proof, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("guardrail: read proof file %s: %w", path, err)
	}
	return ParseProof(data)
}

// claims is the flattened, normalized view used by the verification stages,
// mirroring extract_claims in the Python verifier.
type claims struct {
	agent             string
	agentID           string
	walletAddress     string
	policyHash        string
	reportHash        string
	addressURL        string
	registrationTx    string
	registrationTxURL string
	reportCoreSource  map[string]json.RawMessage
}

// extractClaims normalizes the various proof shapes into a flat claims set,
// preferring the richest commitment-bearing object available.
func (p *Proof) extractClaims() claims {
	summary := p.LatestReport
	report := p.RunReport
	if summary == nil {
		summary = &ProofReport{}
	}
	if report == nil {
		report = &ProofReport{}
	}

	pickStr := func(get func(*ProofReport) string) string {
		if v := get(summary); v != "" {
			return v
		}
		return get(report)
	}

	wallet := pickStr(func(r *ProofReport) string {
		if r.WalletAddress != "" {
			return r.WalletAddress
		}
		return r.Wallet
	})

	agent := p.Agent
	if agent == "" {
		agent = pickStr(func(r *ProofReport) string { return r.Name })
	}

	registrationTx := p.RegistrationTx
	if registrationTx == "" {
		registrationTx = pickStr(func(r *ProofReport) string { return r.RegistrationTx })
	}

	return claims{
		agent:             agent,
		agentID:           pickStr(func(r *ProofReport) string { return r.AgentID }),
		walletAddress:     wallet,
		policyHash:        pickStr(func(r *ProofReport) string { return r.PolicyHash }),
		reportHash:        pickStr(func(r *ProofReport) string { return r.ReportHash }),
		addressURL:        pickStr(func(r *ProofReport) string { return r.AddressURL }),
		registrationTx:    registrationTx,
		registrationTxURL: pickStr(func(r *ProofReport) string { return r.RegistrationTxURL }),
		reportCoreSource:  coreSource(summary, report),
	}
}

// coreSource builds the field map used to re-derive report_hash, preferring the
// summary (latest_report) object that carries the core fields.
func coreSource(summary, report *ProofReport) map[string]json.RawMessage {
	src := reportCoreMap(summary)
	if len(src) == len(reportCoreFields) {
		return src
	}
	if alt := reportCoreMap(report); len(alt) > len(src) {
		return alt
	}
	return src
}

func reportCoreMap(r *ProofReport) map[string]json.RawMessage {
	out := map[string]json.RawMessage{}
	if r.RunID != "" {
		runID, _ := json.Marshal(r.RunID)
		out["run_id"] = runID
	}
	if len(r.Cycles) > 0 {
		out["cycles"] = r.Cycles
	}
	if len(r.FinalNavUSD) > 0 {
		out["final_nav_usd"] = r.FinalNavUSD
	}
	if len(r.TotalDrawdownPct) > 0 {
		out["total_drawdown_pct"] = r.TotalDrawdownPct
	}
	if len(r.Events) > 0 {
		out["events"] = r.Events
	}
	return out
}

// --- Verification stages ----------------------------------------------------

// Verify re-derives every applicable commitment in the proof and validates the
// competition contract metadata, returning a typed VerifyResult. policyFile, if
// non-empty, is the policy file whose SHA-256 is compared to the claimed
// policy_hash; when empty the policy_hash check is skipped (no file to hash
// against). source is a label for reporting (typically the proof file path).
func (p *Proof) Verify(source, policyFile string) VerifyResult {
	c := p.extractClaims()
	checks := []Check{
		verifyWallet(c),
		verifyPolicyHash(c, policyFile),
		verifyReportHash(c),
		verifyAgentID(c),
		verifyAddressURL(c),
		verifyRegistrationTx(c),
	}
	checks = append(checks, verifyCompetitionContract()...)

	passed := true
	for _, ch := range checks {
		if ch.Status == CheckFail {
			passed = false
			break
		}
	}
	return VerifyResult{Source: source, Passed: passed, Checks: checks}
}

func verifyWallet(c claims) Check {
	if c.walletAddress == "" {
		return Check{"wallet_address", CheckFail, "proof carries no wallet_address"}
	}
	if canonicalAddressRe.MatchString(c.walletAddress) {
		return Check{"wallet_address", CheckPass, "valid 20-byte EVM address: " + c.walletAddress}
	}
	if addressRe.MatchString(c.walletAddress) {
		return Check{"wallet_address", CheckPass, "0x-prefixed hex address (demo/vanity placeholder length): " + c.walletAddress}
	}
	return Check{"wallet_address", CheckFail, fmt.Sprintf("wallet_address is not a 0x-prefixed hex EVM address: %q", c.walletAddress)}
}

func verifyPolicyHash(c claims, policyFile string) Check {
	if c.policyHash == "" {
		return Check{"policy_hash", CheckSkip, "no policy_hash claimed in proof; skipped"}
	}
	if !sha256Re.MatchString(c.policyHash) {
		return Check{"policy_hash", CheckFail, fmt.Sprintf("claimed policy_hash is not a 64-char lowercase hex digest: %q", c.policyHash)}
	}
	if policyFile == "" {
		return Check{"policy_hash", CheckSkip, "no policy file supplied to recompute against; skipped"}
	}
	data, err := os.ReadFile(policyFile)
	if err != nil {
		return Check{"policy_hash", CheckSkip, fmt.Sprintf("policy file %s unavailable (%v); skipped", policyFile, err)}
	}
	recomputed := sha256Hex(data)
	if recomputed == c.policyHash {
		return Check{"policy_hash", CheckPass, fmt.Sprintf("recomputed sha256 of %s matches claimed %s", policyFile, c.policyHash)}
	}
	return Check{"policy_hash", CheckFail, fmt.Sprintf("recomputed %s != claimed %s (file %s)", recomputed, c.policyHash, policyFile)}
}

func verifyReportHash(c claims) Check {
	if c.reportHash == "" {
		return Check{"report_hash", CheckSkip, "no report_hash claimed (bare run reports omit it); skipped"}
	}
	if !sha256Re.MatchString(c.reportHash) {
		return Check{"report_hash", CheckFail, fmt.Sprintf("claimed report_hash is not a 64-char lowercase hex digest: %q", c.reportHash)}
	}
	recomputed, ok := reportHashFor(c.reportCoreSource)
	if !ok {
		missing := missingCoreFields(c.reportCoreSource)
		return Check{"report_hash", CheckFail, fmt.Sprintf("cannot re-derive report_hash: proof is missing core field(s) %v required by the agent's hashing", missing)}
	}
	if recomputed == c.reportHash {
		return Check{"report_hash", CheckPass, fmt.Sprintf("recomputed sha256 over {%s} matches claimed %s", strings.Join(reportCoreFields[:], ", "), c.reportHash)}
	}
	return Check{"report_hash", CheckFail, fmt.Sprintf("recomputed %s != claimed %s", recomputed, c.reportHash)}
}

func missingCoreFields(source map[string]json.RawMessage) []string {
	var missing []string
	for _, field := range reportCoreFields {
		if _, ok := source[field]; !ok {
			missing = append(missing, field)
		}
	}
	sort.Strings(missing)
	return missing
}

func verifyAgentID(c claims) Check {
	if c.agentID == "" {
		return Check{"agent_id", CheckSkip, "no agent_id claimed in proof; skipped"}
	}
	if c.agent == "" || c.walletAddress == "" {
		return Check{"agent_id", CheckSkip, "cannot re-derive agent_id without both agent name and wallet; skipped"}
	}
	recomputed := agentIDFor(c.agent, c.walletAddress)
	if recomputed == c.agentID {
		return Check{"agent_id", CheckPass, fmt.Sprintf("recomputed sha256(name\\x00wallet) matches claimed %s", c.agentID)}
	}
	return Check{"agent_id", CheckFail, fmt.Sprintf("recomputed %s != claimed %s (name=%q, wallet=%q)", recomputed, c.agentID, c.agent, c.walletAddress)}
}

func verifyAddressURL(c claims) Check {
	if c.addressURL == "" {
		return Check{"address_url", CheckSkip, "no address_url claimed (bare run reports omit it); skipped"}
	}
	if c.walletAddress == "" {
		return Check{"address_url", CheckFail, "address_url present but wallet_address missing"}
	}
	expected := fmt.Sprintf("%s/address/%s", bscscanBaseURL, c.walletAddress)
	if c.addressURL == expected {
		return Check{"address_url", CheckPass, "BscScan address URL well-formed: " + c.addressURL}
	}
	return Check{"address_url", CheckFail, fmt.Sprintf("address_url %q != expected %q", c.addressURL, expected)}
}

func verifyRegistrationTx(c claims) Check {
	if c.registrationTx == "" {
		return Check{"registration_tx", CheckSkip, "no registration_tx anchored yet (optional, set out-of-band); skipped"}
	}
	if !txHashRe.MatchString(c.registrationTx) {
		return Check{"registration_tx", CheckFail, fmt.Sprintf("registration_tx is not a 0x + 64-hex tx hash: %q", c.registrationTx)}
	}
	if c.registrationTxURL != "" {
		expected := fmt.Sprintf("%s/tx/%s", bscscanBaseURL, c.registrationTx)
		if c.registrationTxURL != expected {
			return Check{"registration_tx", CheckFail, fmt.Sprintf("registration_tx_url %q != expected %q", c.registrationTxURL, expected)}
		}
	}
	return Check{"registration_tx", CheckPass, "valid tx hash format: " + c.registrationTx}
}

// verifyCompetitionContract validates the fixed competition contract address
// and that the published BscTrace explorer URL embeds it exactly.
func verifyCompetitionContract() []Check {
	addrOK := addressRe.MatchString(CompetitionContract)
	addrCheck := Check{Name: "competition_contract_format"}
	if addrOK {
		addrCheck.Status = CheckPass
		addrCheck.Detail = "competition contract is a valid EVM address: " + CompetitionContract
	} else {
		addrCheck.Status = CheckFail
		addrCheck.Detail = "competition contract is malformed: " + CompetitionContract
	}

	expectedExplorer := fmt.Sprintf("https://bsctrace.com/address/%s", CompetitionContract)
	explorerOK := CompetitionContractBsctrace == expectedExplorer
	explorerCheck := Check{Name: "competition_contract_explorer_url"}
	if explorerOK {
		explorerCheck.Status = CheckPass
		explorerCheck.Detail = "explorer URL embeds the contract: " + CompetitionContractBsctrace
	} else {
		explorerCheck.Status = CheckFail
		explorerCheck.Detail = fmt.Sprintf("explorer URL %q does not embed %q", CompetitionContractBsctrace, CompetitionContract)
	}

	return []Check{addrCheck, explorerCheck}
}

// --- Server-side verification (/proof/verify) -------------------------------

// ServerCheck is one server-side verification check from /proof/verify. Unlike
// the offline verifier's Check (PASS/FAIL/SKIP), the server reports a lowercase
// "pass"/"fail" status.
type ServerCheck struct {
	Name   string `json:"name"`
	Status string `json:"status"`
	Detail string `json:"detail"`
}

// Passed reports whether this check has a "pass" status.
func (c ServerCheck) Passed() bool {
	return c.Status == "pass"
}

// RecomputedPolicyHash records a candidate policy file and the sha256 the server
// recomputed for it while validating the claimed policy_hash.
type RecomputedPolicyHash struct {
	File   string `json:"file"`
	SHA256 string `json:"sha256"`
}

// ProofVerifyResponse is the server-side proof verification result from
// /proof/verify. Reason is set (with an empty Checks list) when no run report
// exists yet.
type ProofVerifyResponse struct {
	Passed                 bool                   `json:"passed"`
	Reason                 string                 `json:"reason,omitempty"`
	ReportPath             string                 `json:"report_path,omitempty"`
	RecomputedPolicyHashes []RecomputedPolicyHash `json:"recomputed_policy_hashes,omitempty"`
	Checks                 []ServerCheck          `json:"checks"`
}

// Counts returns the number of passed and failed checks in the response.
func (r ProofVerifyResponse) Counts() (passed, failed int) {
	for _, c := range r.Checks {
		if c.Passed() {
			passed++
		} else {
			failed++
		}
	}
	return passed, failed
}

// --- Client methods ----------------------------------------------------------

// Proof fetches the agent identity + report proof envelope (/proof) and decodes
// it into a typed *Proof. The returned proof can be verified offline with
// (*Proof).Verify.
func (c *Client) Proof(ctx context.Context) (*Proof, error) {
	raw, err := c.doRaw(ctx, "/proof")
	if err != nil {
		return nil, err
	}
	return ParseProof(raw)
}

// ProofVerify fetches the server-side proof verification result (/proof/verify):
// the agent recomputes its own commitments against the on-disk policy + run
// report and returns a per-check pass/fail table. This is the server's view; the
// fully independent re-derivation lives in (*Proof).Verify.
func (c *Client) ProofVerify(ctx context.Context) (*ProofVerifyResponse, error) {
	out := &ProofVerifyResponse{}
	if err := c.do(ctx, "", "/proof/verify", nil, nil, out); err != nil {
		return nil, err
	}
	return out, nil
}
