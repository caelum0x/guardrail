{{/*
Expand the name of the chart.
*/}}
{{- define "guardrail.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create a fully qualified app name.
*/}}
{{- define "guardrail.fullname" -}}
{{- if .Values.fullnameOverride -}}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- $name := default .Chart.Name .Values.nameOverride -}}
{{- if contains $name .Release.Name -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" -}}
{{- else -}}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" -}}
{{- end -}}
{{- end -}}
{{- end -}}

{{/*
Chart name and version label value.
*/}}
{{- define "guardrail.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Common labels applied to every resource.
*/}}
{{- define "guardrail.labels" -}}
helm.sh/chart: {{ include "guardrail.chart" . }}
{{ include "guardrail.selectorLabels" . }}
app.kubernetes.io/part-of: guardrail-alpha
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{/*
Selector labels (stable subset used for matchLabels / Service selectors).
*/}}
{{- define "guardrail.selectorLabels" -}}
app.kubernetes.io/name: {{ include "guardrail.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
Core pod name + selector (agent + api + exporter + monitor co-located).
*/}}
{{- define "guardrail.core.name" -}}
{{- printf "%s-core" (include "guardrail.fullname" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "guardrail.core.selectorLabels" -}}
app.kubernetes.io/name: {{ include "guardrail.name" . }}-core
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
Dashboard name + selector.
*/}}
{{- define "guardrail.dashboard.name" -}}
{{- printf "%s-dashboard" (include "guardrail.fullname" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "guardrail.dashboard.selectorLabels" -}}
app.kubernetes.io/name: {{ include "guardrail.name" . }}-dashboard
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{/*
api / exporter Service names (select into the core pod).
*/}}
{{- define "guardrail.api.name" -}}
{{- printf "%s-api" (include "guardrail.fullname" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "guardrail.exporter.name" -}}
{{- printf "%s-exporter" (include "guardrail.fullname" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Alert relay name + selector (optional component; reads the api /alerts feed and
forwards to chat/email sinks). See integrations/alert-relay.
*/}}
{{- define "guardrail.alertRelay.name" -}}
{{- printf "%s-alert-relay" (include "guardrail.fullname" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "guardrail.alertRelay.selectorLabels" -}}
app.kubernetes.io/name: {{ include "guardrail.name" . }}-alert-relay
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}
