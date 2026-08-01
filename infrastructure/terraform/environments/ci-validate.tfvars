# CI-only variable file used exclusively for `terraform validate`.
# Values are syntactically valid placeholders — never deployed.

environment           = "dev"
aws_region            = "us-east-1"
vpc_cidr_block        = "10.0.0.0/16"
db_name               = "predictiq"
db_username           = "predictiqadmin"
db_password           = "CiValidate!Test#Placeholder2024XYZ"
db_instance_class     = "db.t3.micro"
allocated_storage     = 20
backup_retention_days = 7
redis_node_type       = "cache.t3.micro"
redis_num_nodes       = 1
redis_engine_version  = "7.0"
redis_auth_token      = "CiValidate!Redis#Placeholder2024XYZ"
api_image_uri         = "123456789012.dkr.ecr.us-east-1.amazonaws.com/predictiq:ci-validate"
api_container_port    = 8080
api_desired_count     = 1
api_cpu               = 256
api_memory            = 512
