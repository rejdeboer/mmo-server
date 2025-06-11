FROM golang:1.22.1-alpine AS build-stage

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY ./ ./

RUN CGO_ENABLED=0 go build -o main ./cmd/web-server/main.go

FROM alpine 

RUN apk --update add ca-certificates

COPY --from=build-stage /app/main /app/main
COPY --from=build-stage /app/configuration /app/configuration

EXPOSE 8000

WORKDIR /app

USER root

ENTRYPOINT ["/app/main"]
