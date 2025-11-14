{{/*
Expand the name of the chart.
*/}}
{{- define "llm-orchestrator.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
We truncate at 63 chars because some Kubernetes name fields are limited to this (by the DNS naming spec).
If release name contains chart name it will be used as a full name.
*/}}
{{- define "llm-orchestrator.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "llm-orchestrator.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "llm-orchestrator.labels" -}}
helm.sh/chart: {{ include "llm-orchestrator.chart" . }}
{{ include "llm-orchestrator.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/part-of: llm-orchestrator
app.kubernetes.io/component: orchestrator
{{- if .Values.global.environment }}
environment: {{ .Values.global.environment }}
{{- end }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "llm-orchestrator.selectorLabels" -}}
app.kubernetes.io/name: {{ include "llm-orchestrator.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Create the name of the service account to use
*/}}
{{- define "llm-orchestrator.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "llm-orchestrator.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
PostgreSQL host
*/}}
{{- define "llm-orchestrator.postgresql.host" -}}
{{- if .Values.postgresql.enabled }}
{{- printf "%s-postgresql" .Release.Name }}
{{- else }}
{{- .Values.postgresql.externalHost }}
{{- end }}
{{- end }}

{{/*
PostgreSQL port
*/}}
{{- define "llm-orchestrator.postgresql.port" -}}
{{- if .Values.postgresql.enabled }}
{{- default 5432 5432 }}
{{- else }}
{{- .Values.postgresql.externalPort | default 5432 }}
{{- end }}
{{- end }}

{{/*
PostgreSQL database
*/}}
{{- define "llm-orchestrator.postgresql.database" -}}
{{- if .Values.postgresql.enabled }}
{{- .Values.postgresql.auth.database }}
{{- else }}
{{- .Values.postgresql.externalDatabase }}
{{- end }}
{{- end }}

{{/*
PostgreSQL username
*/}}
{{- define "llm-orchestrator.postgresql.username" -}}
{{- if .Values.postgresql.enabled }}
{{- .Values.postgresql.auth.username }}
{{- else }}
{{- .Values.postgresql.externalUsername }}
{{- end }}
{{- end }}

{{/*
PostgreSQL secret name
*/}}
{{- define "llm-orchestrator.postgresql.secretName" -}}
{{- if .Values.postgresql.enabled }}
{{- if .Values.postgresql.auth.existingSecret }}
{{- .Values.postgresql.auth.existingSecret }}
{{- else }}
{{- printf "%s-postgresql" .Release.Name }}
{{- end }}
{{- else }}
{{- .Values.postgresql.externalSecretName }}
{{- end }}
{{- end }}

{{/*
Redis host
*/}}
{{- define "llm-orchestrator.redis.host" -}}
{{- if .Values.redis.enabled }}
{{- printf "%s-redis-master" .Release.Name }}
{{- else }}
{{- .Values.redis.externalHost }}
{{- end }}
{{- end }}

{{/*
Redis port
*/}}
{{- define "llm-orchestrator.redis.port" -}}
{{- if .Values.redis.enabled }}
{{- default 6379 6379 }}
{{- else }}
{{- .Values.redis.externalPort | default 6379 }}
{{- end }}
{{- end }}

{{/*
Redis secret name
*/}}
{{- define "llm-orchestrator.redis.secretName" -}}
{{- if .Values.redis.enabled }}
{{- if .Values.redis.auth.existingSecret }}
{{- .Values.redis.auth.existingSecret }}
{{- else }}
{{- printf "%s-redis" .Release.Name }}
{{- end }}
{{- else }}
{{- .Values.redis.externalSecretName }}
{{- end }}
{{- end }}

{{/*
Return the proper image name
*/}}
{{- define "llm-orchestrator.image" -}}
{{- $tag := .Values.image.tag | default .Chart.AppVersion -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end }}

{{/*
Return the proper Docker Image Registry Secret Names
*/}}
{{- define "llm-orchestrator.imagePullSecrets" -}}
{{- if .Values.imagePullSecrets }}
imagePullSecrets:
{{- range .Values.imagePullSecrets }}
  - name: {{ . }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Return true if a secret object should be created for LLM providers
*/}}
{{- define "llm-orchestrator.createSecret" -}}
{{- if not .Values.secrets.externalSecrets.enabled }}
{{- true }}
{{- end }}
{{- end }}

{{/*
Get the secrets name
*/}}
{{- define "llm-orchestrator.secretName" -}}
{{- if .Values.secrets.externalSecrets.enabled }}
{{- printf "%s-external-secrets" (include "llm-orchestrator.fullname" .) }}
{{- else }}
{{- printf "%s-secrets" (include "llm-orchestrator.fullname" .) }}
{{- end }}
{{- end }}

{{/*
Get the ConfigMap name
*/}}
{{- define "llm-orchestrator.configMapName" -}}
{{- printf "%s-config" (include "llm-orchestrator.fullname" .) }}
{{- end }}

{{/*
Validate values
*/}}
{{- define "llm-orchestrator.validateValues" -}}
{{- if and (not .Values.postgresql.enabled) (not .Values.postgresql.externalHost) -}}
llm-orchestrator: PostgreSQL
    You must enable PostgreSQL (postgresql.enabled=true) or provide an external PostgreSQL host (postgresql.externalHost)
{{- end }}
{{- if and (not .Values.redis.enabled) (not .Values.redis.externalHost) -}}
llm-orchestrator: Redis
    You must enable Redis (redis.enabled=true) or provide an external Redis host (redis.externalHost)
{{- end }}
{{- if and .Values.ingress.enabled (not .Values.ingress.hosts) -}}
llm-orchestrator: Ingress
    You must provide at least one host when ingress is enabled
{{- end }}
{{- if and .Values.autoscaling.enabled (lt (.Values.autoscaling.minReplicas | int) 1) -}}
llm-orchestrator: Autoscaling
    Minimum replicas must be at least 1
{{- end }}
{{- end }}

{{/*
Compile all warnings into a single message
*/}}
{{- define "llm-orchestrator.validateValues.warnings" -}}
{{- $warnings := list -}}
{{- if and .Values.postgresql.enabled (not .Values.postgresql.auth.password) (not .Values.postgresql.auth.existingSecret) -}}
{{- $warnings = append $warnings "WARNING: PostgreSQL password not set. A random password will be generated." -}}
{{- end }}
{{- if and .Values.redis.enabled .Values.redis.auth.enabled (not .Values.redis.auth.password) (not .Values.redis.auth.existingSecret) -}}
{{- $warnings = append $warnings "WARNING: Redis password not set. A random password will be generated." -}}
{{- end }}
{{- if not .Values.monitoring.enabled -}}
{{- $warnings = append $warnings "WARNING: Monitoring is disabled. Consider enabling for production deployments." -}}
{{- end }}
{{- if not .Values.networkPolicy.enabled -}}
{{- $warnings = append $warnings "WARNING: Network policies are disabled. Consider enabling for production security." -}}
{{- end }}
{{- if $warnings -}}
{{- range $warnings }}
{{ . }}
{{- end }}
{{- end }}
{{- end }}
