# Alelysee - Dioxus Fullstack App Makefile
# ===========================================

.PHONY: help setup build test clean dev deploy docker-build docker-push db-setup aws-monitoring aws-status aws-monitoring-status aws-resources aws-alb-dns aws-cleanup-unused aws-cleanup-auto aws-cleanup-force

# Variables
ENVIRONMENT ?= dev

# Default target
help: ## Show this help message
	@echo "Alelysee - Dioxus Fullstack App"
	@echo "==============================="
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2}'

# Setup and Development
# =====================

setup: ## Install dependencies and setup development environment
	@echo "Setting up development environment..."
	@if ! command -v jq >/dev/null 2>&1; then echo "Installing jq..."; brew install jq; fi
	@if ! command -v aws >/dev/null 2>&1; then echo "Installing AWS CLI..."; brew install awscli; fi
	cargo install dioxus-cli --locked
	cargo install cargo-watch

setup-aws: ## Setup AWS CLI and configure credentials
	@echo "Setting up AWS CLI..."
	aws configure

install-deps: ## Install all Cargo dependencies
	cargo fetch

# Development Commands
# ===================

dev: ## Start development server (fullstack mode)
	cd packages/web && dx serve -p web --web --fullstack

dev-server: ## Start only the server component
	cd packages/web && dx serve -p server --server --fullstack

dev-client: ## Start only the client component
	cd packages/web && dx serve -p web --web

dev-desktop: ## Start desktop app in development
	cd packages/desktop && dx serve -p desktop --desktop

dev-mobile: ## Start mobile app in development
	cd packages/mobile && dx serve -p mobile --mobile

# Building
# ========

build: ## Build all packages in debug mode
	cargo build --workspace

build-release: ## Build all packages in release mode
	cargo build --workspace --release

build-web: ## Build web package
	cargo build --package web --release

build-server: ## Build server binary
	cargo build --package web --release --features server

build-desktop: ## Build desktop app
	cargo build --package desktop --release

build-mobile: ## Build mobile app
	cargo build --package mobile --release

# Testing
# =======

test: ## Run all tests
	cargo test --workspace

test-api: ## Run API-specific tests
	cargo test --package api

test-ui: ## Run UI-specific tests
	cargo test --package ui

test-web: ## Run web-specific tests
	cargo test --package web

# Database
# ========

db-migrate: ## Run database migrations
	cargo run --package api --bin migrate

db-setup: ## Setup database (requires DATABASE_URL)
	@echo "Setting up database..."
	@if [ -z "$$DATABASE_URL" ]; then echo "DATABASE_URL not set"; exit 1; fi
	cargo run --package api --bin migrate

# AWS Setup and Deployment
# ========================

aws-monitoring: ## Setup AWS monitoring and analytics (CloudWatch, health checks, etc.) - REQUIRES DEPLOYED APP
	@echo "Setting up AWS monitoring and analytics..."
	@echo "⚠️  This requires a deployed application with ALB."
	@echo "   Make sure the CloudFormation stack is deployed first."
	@echo ""
	./scripts/aws_monitoring.sh $(ENVIRONMENT)

aws-status: ## Check AWS resources status
	@echo "Checking AWS resources..."
	@echo "Cognito User Pool:"
	aws cognito-idp describe-user-pool --user-pool-id $$(grep COGNITO_USER_POOL_ID .env | cut -d'=' -f2) --region $$(grep AWS_REGION .env | cut -d'=' -f2) --query 'UserPool.Name'
	@echo "S3 Bucket:"
	aws s3 ls s3://$$(grep S3_BUCKET .env | cut -d'=' -f2) --region $$(grep AWS_REGION .env | cut -d'=' -f2) | head -5
	@echo "CloudFront Distribution:"
	aws cloudfront list-distributions --query 'DistributionList.Items[?Comment==`alelysee videos`].DomainName'

aws-monitoring-status: ## Check monitoring and analytics status
	@echo "Checking monitoring status..."
	@echo "CloudWatch Dashboard:"
	aws cloudwatch list-dashboards --query 'DashboardEntries[?DashboardName==`$(APP_NAME)-$(ENVIRONMENT)-dashboard`].DashboardName'
	@echo "Route 53 Health Checks:"
	aws route53 list-health-checks --query 'HealthChecks[?contains(CallerReference, `$(APP_NAME)-$(ENVIRONMENT)`)].Id'
	@echo "CloudWatch Alarms:"
	aws cloudwatch describe-alarms --alarm-name-prefix "$(APP_NAME)-$(ENVIRONMENT)" --query 'MetricAlarms[*].AlarmName'

aws-resources: ## List all AWS resources created by deployment
	@echo "AWS Resources Overview:"
	@echo "======================="
	@echo ""
	@echo "🔍 VPCs:"
	@aws ec2 describe-vpcs --filters "Name=tag:Application,Values=alelysee" --query 'Vpcs[*].[VpcId,Tags[?Key==`Name`].Value|[0],Tags[?Key==`Environment`].Value|[0]]' --output table 2>/dev/null || echo "No VPCs found"
	@echo ""
	@echo "🌐 Subnets:"
	@aws ec2 describe-subnets --filters "Name=tag:Application,Values=alelysee" --query 'Subnets[*].[SubnetId,Tags[?Key==`Name`].Value|[0],AvailabilityZone,CidrBlock]' --output table 2>/dev/null || echo "No subnets found"
	@echo ""
	@echo "🚪 Security Groups:"
	@aws ec2 describe-security-groups --filters "Name=tag:Application,Values=alelysee" --query 'SecurityGroups[*].[GroupId,GroupName,Tags[?Key==`Environment`].Value|[0]]' --output table 2>/dev/null || echo "No security groups found"
	@echo ""
	@echo "🗄️ RDS Instances:"
	@aws rds describe-db-instances --query 'DBInstances[*].[DBInstanceIdentifier,DBInstanceStatus,Endpoint.Address]' --output table 2>/dev/null | grep alelysee || echo "No RDS instances found"
	@echo ""
	@echo "⚖️ Load Balancers:"
	@aws elbv2 describe-load-balancers --query 'LoadBalancers[*].[LoadBalancerName,DNSName,State.Code]' --output table 2>/dev/null | grep alelysee || echo "No load balancers found"
	@echo ""
	@echo "🐳 ECS Clusters:"
	@aws ecs list-clusters --query 'clusterArns[*]' --output text 2>/dev/null | grep alelysee | sed 's|.*/||' | paste -sd "," - | sed 's/^/ECS clusters: /' || echo "No ECS clusters found"
	@echo ""
	@echo "📦 ECR Repositories:"
	@aws ecr describe-repositories --query 'repositories[*].[repositoryName,repositoryUri]' --output table 2>/dev/null | grep alelysee || echo "No ECR repositories found"
	@echo ""
	@echo "🎯 Target Groups:"
	@aws elbv2 describe-target-groups --query 'TargetGroups[*].[TargetGroupName,Protocol,Port]' --output table 2>/dev/null | grep alelysee || echo "No target groups found"

aws-alb-dns: ## Show ALB DNS names for DNS configuration
	@echo "Application Load Balancer DNS Names:"
	@echo "====================================="
	@echo ""
	@echo "For DNS CNAME records in OVH:"
	@echo ""
	@aws elbv2 describe-load-balancers --query 'LoadBalancers[*].[LoadBalancerName,DNSName]' --output table 2>/dev/null | grep alelysee || echo "No ALBs found"
	@echo ""
	@echo "DNS Configuration:"
	@echo "test.alelysee.com → [DEV_ALB_DNS]"
	@echo "app.alelysee.com  → [PROD_ALB_DNS]"

aws-cleanup-unused: ## Remove unused VPCs and networks (CAUTION: destroys resources)
	@echo "⚠️  WARNING: This will delete VPCs and all associated resources!"
	@echo "Only unused VPCs (without active resources) will be deleted."
	@echo ""
	@echo "Checking for unused VPCs..."
	@for vpc_id in $$(aws ec2 describe-vpcs --query 'Vpcs[*].VpcId' --output text); do \
		has_resources=$$(aws ec2 describe-network-interfaces --filters "Name=vpc-id,Values=$$vpc_id" --query 'NetworkInterfaces[*].NetworkInterfaceId' --output text 2>/dev/null | wc -l); \
		if [ "$$has_resources" -eq 0 ]; then \
			vpc_name=$$(aws ec2 describe-vpcs --vpc-ids $$vpc_id --query 'Vpcs[0].Tags[?Key==`Name`].Value|[0]' --output text 2>/dev/null); \
			if [[ "$$vpc_name" != "None" && "$$vpc_name" != "" ]]; then \
				echo "Found unused VPC: $$vpc_id ($$vpc_name)"; \
				read -p "Delete VPC $$vpc_id ($$vpc_name)? (y/N): " confirm; \
				if [[ "$$confirm" == "y" || "$$confirm" == "Y" ]]; then \
					echo "Deleting VPC $$vpc_id..."; \
					aws ec2 delete-vpc --vpc-id $$vpc_id 2>/dev/null && echo "✅ Deleted VPC $$vpc_id" || echo "❌ Failed to delete VPC $$vpc_id"; \
				fi; \
			fi; \
		fi; \
	done
	@echo "Cleanup complete. Run 'make aws-resources' to verify."

aws-cleanup-auto: ## Automatically remove truly unused VPCs (safe)
	@echo "🔍 Finding truly empty VPCs (no subnets or security groups)..."
	@echo "Note: This only deletes VPCs that are completely empty."
	@echo "VPCs with resources need manual cleanup or use 'aws-cleanup-force'."
	@echo ""
	@./scripts/cleanup_unused_vpcs.sh

aws-cleanup-force: ## Force cleanup of all alelysee resources (CAUTION: destroys everything)
	@echo "⚠️  DANGER: This will delete ALL alelysee resources!"
	@echo "This includes VPCs, subnets, security groups, ALBs, ECS clusters, etc."
	@echo ""
	@read -p "Are you SURE you want to delete ALL alelysee resources? Type 'YES' to confirm: " confirm; \
	if [[ "$$confirm" != "YES" ]]; then \
		echo "Aborted."; \
		exit 1; \
	fi; \
	echo ""; \
	echo "🗑️  Starting forced cleanup..."; \
	\
	# Delete ECS services and tasks first \
	echo "1️⃣ Deleting ECS services..."; \
	for cluster_arn in $$(aws ecs list-clusters --query 'clusterArns[*]' --output text 2>/dev/null | xargs -I {} aws ecs describe-clusters --clusters {} --query 'clusters[?contains(clusterName, `alelysee`)].clusterArn' --output text 2>/dev/null); do \
		cluster_name=$$(basename $$cluster_arn); \
		for service_arn in $$(aws ecs list-services --cluster $$cluster_name --query 'serviceArns[*]' --output text 2>/dev/null); do \
			service_name=$$(basename $$service_arn); \
			echo "Deleting ECS service: $$service_name"; \
			aws ecs update-service --cluster $$cluster_name --service $$service_name --desired-count 0 >/dev/null 2>&1; \
			aws ecs delete-service --cluster $$cluster_name --service $$service_name >/dev/null 2>&1; \
		done; \
		echo "Deleting ECS cluster: $$cluster_name"; \
		aws ecs delete-cluster --cluster $$cluster_name >/dev/null 2>&1; \
	done; \
	\
	# Delete ALBs and target groups \
	echo "2️⃣ Deleting ALBs and target groups..."; \
	for alb_arn in $$(aws elbv2 describe-load-balancers --query 'LoadBalancers[?contains(LoadBalancerName, `alelysee`)].LoadBalancerArn' --output text 2>/dev/null); do \
		alb_name=$$(aws elbv2 describe-load-balancers --load-balancer-arns $$alb_arn --query 'LoadBalancers[0].LoadBalancerName' --output text 2>/dev/null); \
		echo "Deleting ALB: $$alb_name"; \
		aws elbv2 delete-load-balancer --load-balancer-arn $$alb_arn >/dev/null 2>&1; \
	done; \
	\
	for tg_arn in $$(aws elbv2 describe-target-groups --query 'TargetGroups[?contains(TargetGroupName, `alelysee`)].TargetGroupArn' --output text 2>/dev/null); do \
		tg_name=$$(aws elbv2 describe-target-groups --target-group-arns $$tg_arn --query 'TargetGroups[0].TargetGroupName' --output text 2>/dev/null); \
		echo "Deleting target group: $$tg_name"; \
		aws elbv2 delete-target-group --target-group-arn $$tg_arn >/dev/null 2>&1; \
	done; \
	\
	# Delete RDS instances \
	echo "3️⃣ Deleting RDS instances..."; \
	for db_id in $$(aws rds describe-db-instances --query 'DBInstances[?contains(DBInstanceIdentifier, `alelysee`)].DBInstanceIdentifier' --output text 2>/dev/null); do \
		echo "Deleting RDS instance: $$db_id"; \
		aws rds delete-db-instance --db-instance-identifier $$db_id --skip-final-snapshot >/dev/null 2>&1; \
	done; \
	\
	# Wait a bit for dependencies to be deleted \
	echo "4️⃣ Waiting for dependencies to be cleaned up..."; \
	sleep 30; \
	\
	# Delete subnets \
	echo "5️⃣ Deleting subnets..."; \
	for subnet_id in $$(aws ec2 describe-subnets --query 'Subnets[?Tags[?Key==`Name` && contains(Value, `alelysee`)]].SubnetId' --output text 2>/dev/null); do \
		subnet_name=$$(aws ec2 describe-subnets --subnet-ids $$subnet_id --query 'Subnets[0].Tags[?Key==`Name`].Value|[0]' --output text 2>/dev/null); \
		echo "Deleting subnet: $$subnet_name"; \
		aws ec2 delete-subnet --subnet-id $$subnet_id >/dev/null 2>&1; \
	done; \
	\
	# Delete security groups \
	echo "6️⃣ Deleting security groups..."; \
	for sg_id in $$(aws ec2 describe-security-groups --query 'SecurityGroups[?Tags[?Key==`Application` && Value==`alelysee`]].GroupId' --output text 2>/dev/null); do \
		sg_name=$$(aws ec2 describe-security-groups --group-ids $$sg_id --query 'SecurityGroups[0].GroupName' --output text 2>/dev/null); \
		echo "Deleting security group: $$sg_name"; \
		aws ec2 delete-security-group --group-id $$sg_id >/dev/null 2>&1; \
	done; \
	\
	# Delete NAT gateways and internet gateways \
	echo "7️⃣ Deleting NAT gateways and internet gateways..."; \
	for nat_id in $$(aws ec2 describe-nat-gateways --query 'NatGateways[?Tags[?Key==`Application` && Value==`alelysee`]].NatGatewayId' --output text 2>/dev/null); do \
		echo "Deleting NAT gateway: $$nat_id"; \
		aws ec2 delete-nat-gateway --nat-gateway-id $$nat_id >/dev/null 2>&1; \
	done; \
	\
	for igw_id in $$(aws ec2 describe-internet-gateways --query 'InternetGateways[?Tags[?Key==`Application` && Value==`alelysee`]].InternetGatewayId' --output text 2>/dev/null); do \
		# Detach from VPC first \
		vpc_id=$$(aws ec2 describe-internet-gateways --internet-gateway-ids $$igw_id --query 'InternetGateways[0].Attachments[0].VpcId' --output text 2>/dev/null); \
		if [[ -n "$$vpc_id" ]]; then \
			aws ec2 detach-internet-gateway --internet-gateway-id $$igw_id --vpc-id $$vpc_id >/dev/null 2>&1; \
		fi; \
		echo "Deleting internet gateway: $$igw_id"; \
		aws ec2 delete-internet-gateway --internet-gateway-id $$igw_id >/dev/null 2>&1; \
	done; \
	\
	# Finally delete VPCs \
	echo "8️⃣ Deleting VPCs..."; \
	for vpc_id in $$(aws ec2 describe-vpcs --query 'Vpcs[?Tags[?Key==`Application` && Value==`alelysee`]].VpcId' --output text 2>/dev/null); do \
		vpc_name=$$(aws ec2 describe-vpcs --vpc-ids $$vpc_id --query 'Vpcs[0].Tags[?Key==`Name`].Value|[0]' --output text 2>/dev/null); \
		echo "Deleting VPC: $$vpc_name"; \
		aws ec2 delete-vpc --vpc-id $$vpc_id >/dev/null 2>&1; \
	done; \
	\
	echo ""; \
	echo "🎉 Forced cleanup complete!"; \
	echo "Note: Some resources may take time to delete completely."; \
	echo "Run 'make aws-resources' in a few minutes to verify."

aws-cleanup-full: ## Comprehensive cleanup - runs safe cleanup then shows remaining resources
	@echo "🧹 Starting comprehensive cleanup..."
	@echo ""
	@echo "1️⃣ Cleaning up truly empty VPCs..."
	$(MAKE) aws-cleanup-auto
	@echo ""
	@echo "2️⃣ Remaining resources after cleanup:"
	$(MAKE) aws-resources

aws-cleanup-stack: ## Clean up entire stack for specific environment and region
	@echo "🗑️  Starting stack cleanup..."
	@if [ -z "$$ENVIRONMENT" ]; then echo "Please set ENVIRONMENT variable (dev/prod)"; exit 1; fi
	@if [ -z "$$REGION" ]; then echo "Please set REGION variable (e.g., eu-west-3)"; exit 1; fi
	@echo "Environment: $$ENVIRONMENT"
	@echo "Region: $$REGION"
	./scripts/aws_cleanup.sh $$ENVIRONMENT $$REGION

# Docker Commands
# ===============

docker-build: ## Build Docker image
	docker build -t alelysee:latest .

docker-run: ## Run Docker container locally
	docker run -p 8080:8080 --env-file .env alelysee:latest

docker-push: ## Push Docker image to registry (requires DOCKER_REGISTRY)
	@if [ -z "$$DOCKER_REGISTRY" ]; then echo "DOCKER_REGISTRY not set"; exit 1; fi
	@echo "Authenticating with container registry..."
	@if echo "$$DOCKER_REGISTRY" | grep -q "amazonaws.com"; then \
		aws ecr get-login-password --region $$(echo "$$DOCKER_REGISTRY" | sed 's/.*\.dkr\.ecr\.\([^.]*\).*/\1/') | docker login --username AWS --password-stdin $$DOCKER_REGISTRY; \
	else \
		echo "Non-ECR registry detected. Please ensure you're authenticated manually."; \
	fi
	docker tag alelysee:latest $$DOCKER_REGISTRY/$$ECR_REPOSITORY:latest
	docker push $$DOCKER_REGISTRY/$$ECR_REPOSITORY:latest

health-check: ## Run health checks
	@echo "Running health checks..."
	@if curl -f http://localhost:8080/api/health >/dev/null 2>&1; then echo "✅ Server health check passed"; else echo "❌ Server health check failed"; fi

logs: ## Show application logs (requires running container)
	docker logs $$(docker ps -q --filter ancestor=alelysee)

# Cleanup
# =======

clean: ## Clean build artifacts
	cargo clean
	rm -rf target/

clean-docker: ## Clean Docker images and containers
	docker system prune -f
	docker image rm alelysee:latest 2>/dev/null || true

clean-all: clean clean-docker ## Clean everything

# Utility Commands
# ================

fmt: ## Format code
	cargo fmt --all

fmt-check: ## Check code formatting
	cargo fmt --all -- --check

lint: ## Run clippy linter
	cargo clippy --all-targets --all-features -- -D warnings

check: ## Check code without building
	cargo check --workspace

update: ## Update dependencies
	cargo update

audit: ## Audit dependencies for security vulnerabilities
	cargo audit

# Environment Management
# ======================

env-check: ## Check environment variables
	@echo "Checking environment variables..."
	@if [ ! -f .env ]; then echo "❌ .env file not found"; exit 1; fi
	@grep -v '^#' .env | grep -v '^$$' | while read line; do \
		key=$$(echo $$line | cut -d'=' -f1); \
		value=$$(echo $$line | cut -d'=' -f2-); \
		if [ -z "$$value" ]; then echo "❌ $$key is not set"; else echo "✅ $$key = $$value"; fi; \
	done

env-template: ## Generate .env template from env.example
	cp env.example .env
	@echo "✅ .env template created. Please fill in the required values."

# Information
# ===========

info: ## Show project information
	@echo "Alelysee - Dioxus Fullstack App"
	@echo "================================="
	@echo "Packages:"
	@cargo tree --workspace --depth 0 | grep -E "(api|web|desktop|mobile|ui)"
	@echo ""
	@echo "Environment:"
	@if [ -f .env ]; then echo "✅ .env file exists"; else echo "❌ .env file missing"; fi
	@echo "Rust version: $$(rustc --version)"
	@echo "Cargo version: $$(cargo --version)"
	@echo "DX version: $$(dx --version 2>/dev/null || echo 'Not installed')"
