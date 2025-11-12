#!/bin/bash

export RUST_LOG="INFO"
export ROUTE_HEALTH_PATH="/health"
export ROUTE_HEALTH_METHOD="GET"
export ROUTE_APP_PATH="/app"
export ROUTE_APP_METHOD="POST"
export ROUTE_APP_LATENCY="100"
export ROUTE_APPLIST_PATH="/apps"
export ROUTE_APPLIST_METHOD="GET"
export ROUTE_APPGET_PATH="/app/{id}"
export ROUTE_APPGET_METHOD="GET"
