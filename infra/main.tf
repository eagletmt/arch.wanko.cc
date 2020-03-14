resource "aws_s3_bucket" "arch" {
  bucket = "arch.wanko.cc"

  website {
    index_document = "index.html"
    error_document = "404.html"
  }
}

data "aws_iam_policy_document" "arch" {
  statement {
    principals {
      type        = "AWS"
      identifiers = ["*"]
    }
    actions   = ["s3:GetObject"]
    resources = ["${aws_s3_bucket.arch.arn}/*"]
  }
}

resource "aws_s3_bucket_policy" "arch" {
  bucket = aws_s3_bucket.arch.id
  policy = data.aws_iam_policy_document.arch.json
}

data "aws_route53_zone" "wanko-cc" {
  name         = "wanko.cc."
  private_zone = false
}

resource "aws_route53_record" "arch" {
  zone_id = data.aws_route53_zone.wanko-cc.zone_id
  name    = "arch.wanko.cc"
  type    = "A"
  alias {
    name                   = aws_s3_bucket.arch.website_domain
    zone_id                = aws_s3_bucket.arch.hosted_zone_id
    evaluate_target_health = false
  }
}

