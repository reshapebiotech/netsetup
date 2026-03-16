group "default" {
  targets = ["machine-setup-test"]
}

target "machine-setup-test" {
  context = "."
  dockerfile = "Dockerfile"
  tags = ["machine-setup-test:latest"]
  output = ["type=docker"]
}
