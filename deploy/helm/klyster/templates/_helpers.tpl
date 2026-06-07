{{/*
Expand the chart name.
*/}}
{{- define "klyster.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "klyster.fullname" -}}
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

{{- define "klyster.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "klyster.labels" -}}
helm.sh/chart: {{ include "klyster.chart" . }}
{{ include "klyster.selectorLabels" . }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end -}}

{{- define "klyster.selectorLabels" -}}
app.kubernetes.io/name: {{ include "klyster.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end -}}

{{- define "klyster.serviceAccountName" -}}
{{- if .Values.serviceAccount.create -}}
{{- default (include "klyster.fullname" .) .Values.serviceAccount.name -}}
{{- else -}}
{{- default "default" .Values.serviceAccount.name -}}
{{- end -}}
{{- end -}}

{{- define "klyster.appImage" -}}
{{- $tag := default .Chart.AppVersion .Values.image.tag -}}
{{- printf "%s:%s" .Values.image.repository $tag -}}
{{- end -}}

{{- define "klyster.seerImage" -}}
{{- $tag := default .Chart.AppVersion .Values.seer.image.tag -}}
{{- printf "%s:%s" .Values.seer.image.repository $tag -}}
{{- end -}}

{{- define "klyster.postgresName" -}}
{{- printf "%s-postgres" (include "klyster.fullname" .) | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "klyster.postgresSecretName" -}}
{{- if .Values.database.postgres.internal.enabled -}}
{{- default (printf "%s-postgres" (include "klyster.fullname" .)) .Values.database.postgres.internal.auth.existingSecret -}}
{{- else -}}
{{- default (printf "%s-postgres" (include "klyster.fullname" .)) .Values.database.postgres.external.existingSecret -}}
{{- end -}}
{{- end -}}

{{- define "klyster.postgresPasswordKey" -}}
{{- if .Values.database.postgres.internal.enabled -}}
{{- .Values.database.postgres.internal.auth.existingSecretPasswordKey -}}
{{- else -}}
{{- .Values.database.postgres.external.existingSecretPasswordKey -}}
{{- end -}}
{{- end -}}

{{- define "klyster.postgresUrlKey" -}}
{{- if .Values.database.postgres.internal.enabled -}}
{{- .Values.database.postgres.internal.auth.existingSecretUrlKey -}}
{{- else -}}
{{- .Values.database.postgres.external.existingSecretUrlKey -}}
{{- end -}}
{{- end -}}

{{- define "klyster.postgresHost" -}}
{{- if .Values.database.postgres.internal.enabled -}}
{{- include "klyster.postgresName" . -}}
{{- else -}}
{{- required "database.postgres.external.host is required when database.type=postgres and internal PostgreSQL is disabled" .Values.database.postgres.external.host -}}
{{- end -}}
{{- end -}}

{{- define "klyster.postgresUsername" -}}
{{- if .Values.database.postgres.internal.enabled -}}
{{- .Values.database.postgres.internal.auth.username -}}
{{- else -}}
{{- .Values.database.postgres.external.username -}}
{{- end -}}
{{- end -}}

{{- define "klyster.postgresDatabase" -}}
{{- if .Values.database.postgres.internal.enabled -}}
{{- .Values.database.postgres.internal.auth.database -}}
{{- else -}}
{{- .Values.database.postgres.external.database -}}
{{- end -}}
{{- end -}}

{{- define "klyster.postgresPort" -}}
{{- if .Values.database.postgres.internal.enabled -}}
5432
{{- else -}}
{{- .Values.database.postgres.external.port -}}
{{- end -}}
{{- end -}}

{{- define "klyster.postgresSslMode" -}}
{{- if .Values.database.postgres.internal.enabled -}}
disable
{{- else -}}
{{- .Values.database.postgres.external.sslMode -}}
{{- end -}}
{{- end -}}
