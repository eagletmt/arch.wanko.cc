provider "aws" {
  region = "ap-northeast-1"
}

terraform {
  backend "s3" {
    bucket = "terraform-wanko-cc"
    key    = "arch.tfstate"
    region = "ap-northeast-1"
  }
}

