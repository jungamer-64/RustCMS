#!/bin/bash

# Production build script for CMS Backend

set -e

echo "🔧 Building CMS Backend for production..."

# Clean previous builds
echo "🧹 Cleaning previous builds..."
cargo clean

# Update dependencies
echo "📦 Updating dependencies..."
cargo update

# Run tests
echo "🧪 Running tests..."
cargo test --release

# Build for production
echo "🏗️  Building for production..."
cargo build --release

# Copy binary to deployment directory
echo "📁 Copying binary..."
mkdir -p deploy
cp target/release/cms-backend deploy/
cp -r migrations deploy/
cp .env.example deploy/.env

echo "✅ Build completed successfully!"
echo "📂 Deployment files are in the 'deploy' directory"
echo ""
echo "🚀 To run in production:"
echo "   cd deploy"
echo "   ./cms-backend"
