# NOTE: This image is used to apply DB migrations inside a K8S cluster
FROM golang:1.22.1-alpine AS build-stage

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY ./ ./

RUN CGO_ENABLED=0 go build -o main ./db/migration.go

FROM alpine 

RUN apk --update add ca-certificates

COPY --from=build-stage /app/main /app/main
COPY --from=build-stage /app/configuration /app/configuration
COPY --from=build-stage /app/db/migrations /app/migrations

WORKDIR /app

ENTRYPOINT ["/app/main"]
