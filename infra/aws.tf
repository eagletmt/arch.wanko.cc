provider "aws" {
  version = "2.53.0"
  region  = "ap-northeast-1"
}

terraform {
  backend "s3" {
    bucket = "terraform-wanko-cc"
    key    = "arch.tfstate"
    region = "ap-northeast-1"
  }
}

