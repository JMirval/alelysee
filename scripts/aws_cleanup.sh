#!/usr/bin/env bash
set -euo pipefail

# Heliastes AWS Cleanup Script
# ============================
#
# This script cleans up all AWS resources created by the deployment script.
# It removes resources in a safe order to avoid dependency issues.
#
# Prerequisites:
# - AWS CLI configured
# - Appropriate permissions to delete resources
#
# Usage:
#   ./scripts/aws_cleanup.sh [environment] [region]
#
# Environment: dev | prod (default: dev)
# Region: AWS region (default: current AWS_REGION or us-east-1)

ENVIRONMENT="${1:-dev}"
REGION="${2:-}"
APP_NAME="alelysee"
STACK_NAME="${APP_NAME}-${ENVIRONMENT}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() { echo -e "${GREEN}[INFO]${NC} $*" >&2; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*" >&2; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }
log_step() { echo -e "${BLUE}[STEP]${NC} $*" >&2; }

# Check dependencies
need() {
    command -v "$1" >/dev/null 2>&1 || { log_error "Missing dependency: $1"; exit 1; }
}

need aws
need jq

# Get AWS region
get_region() {
    if [[ -n "$REGION" ]]; then
        AWS_REGION="$REGION"
    elif [[ -f ".env" ]]; then
        AWS_REGION=$(grep "AWS_REGION" .env | head -1 | cut -d'=' -f2 | tr -d "'\"")
    fi

    if [[ -z "${AWS_REGION:-}" ]]; then
        AWS_REGION="us-east-1"
        log_warn "No region specified, using default: $AWS_REGION"
    fi

    log_info "Using AWS Region: $AWS_REGION"
}

# Delete ECS service and cluster
cleanup_ecs() {
    log_step "Cleaning up ECS resources..."

    # Stop all running tasks
    if aws ecs list-tasks --cluster "${STACK_NAME}-cluster" --region "$AWS_REGION" >/dev/null 2>&1; then
        TASK_ARNS=$(aws ecs list-tasks --cluster "${STACK_NAME}-cluster" --region "$AWS_REGION" --query 'taskArns' --output text)
        if [[ -n "$TASK_ARNS" ]]; then
            log_info "Stopping running tasks..."
            for task_arn in $TASK_ARNS; do
                aws ecs stop-task --cluster "${STACK_NAME}-cluster" --task "$task_arn" --region "$AWS_REGION" >/dev/null 2>&1 || true
            done
        fi
    fi

    # Delete service
    if aws ecs describe-services --cluster "${STACK_NAME}-cluster" --services "${STACK_NAME}-service" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting ECS service..."
        aws ecs update-service --cluster "${STACK_NAME}-cluster" --service "${STACK_NAME}-service" --desired-count 0 --region "$AWS_REGION" >/dev/null 2>&1 || true
        aws ecs delete-service --cluster "${STACK_NAME}-cluster" --service "${STACK_NAME}-service" --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    # Delete task definitions
    if aws ecs list-task-definitions --family-prefix "${STACK_NAME}-task" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deregistering task definitions..."
        TASK_DEF_ARNS=$(aws ecs list-task-definitions --family-prefix "${STACK_NAME}-task" --region "$AWS_REGION" --query 'taskDefinitionArns' --output text)
        for task_def_arn in $TASK_DEF_ARNS; do
            aws ecs deregister-task-definition --task-definition "$task_def_arn" --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
    fi

    # Delete cluster
    if aws ecs describe-clusters --clusters "${STACK_NAME}-cluster" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting ECS cluster..."
        aws ecs delete-cluster --cluster "${STACK_NAME}-cluster" --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    log_info "ECS cleanup complete"
}

# Delete ALB and target groups
cleanup_alb() {
    log_step "Cleaning up ALB resources..."

    # Find ALB
    ALB_ARN=$(aws elbv2 describe-load-balancers --region "$AWS_REGION" --query "LoadBalancers[?LoadBalancerName=='${STACK_NAME}-alb'].LoadBalancerArn" --output text 2>/dev/null || echo "")

    if [[ -n "$ALB_ARN" ]]; then
        log_info "Deleting ALB..."

        # Delete listeners
        LISTENER_ARNS=$(aws elbv2 describe-listeners --load-balancer-arn "$ALB_ARN" --region "$AWS_REGION" --query 'Listeners[*].ListenerArn' --output text 2>/dev/null || echo "")
        for listener_arn in $LISTENER_ARNS; do
            aws elbv2 delete-listener --listener-arn "$listener_arn" --region "$AWS_REGION" >/dev/null 2>&1 || true
        done

        # Delete ALB
        aws elbv2 delete-load-balancer --load-balancer-arn "$ALB_ARN" --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    # Delete target groups
    TG_ARNS=$(aws elbv2 describe-target-groups --region "$AWS_REGION" --query "TargetGroups[?TargetGroupName=='${STACK_NAME}-tg'].TargetGroupArn" --output text 2>/dev/null || echo "")
    for tg_arn in $TG_ARNS; do
        aws elbv2 delete-target-group --target-group-arn "$tg_arn" --region "$AWS_REGION" >/dev/null 2>&1 || true
    done

    log_info "ALB cleanup complete"
}

# Delete RDS database
cleanup_rds() {
    log_step "Cleaning up RDS resources..."

    # Delete DB instance
    if aws rds describe-db-instances --db-instance-identifier "${STACK_NAME}-db" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting RDS instance..."
        aws rds delete-db-instance --db-instance-identifier "${STACK_NAME}-db" --skip-final-snapshot --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    # Delete DB subnet group
    if aws rds describe-db-subnet-groups --db-subnet-group-name "alelysee-db-subnet-group" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting DB subnet group..."
        aws rds delete-db-subnet-group --db-subnet-group-name "alelysee-db-subnet-group" --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    log_info "RDS cleanup complete"
}

# Delete ECR repositories
cleanup_ecr() {
    log_step "Cleaning up ECR resources..."

    # Delete repository
    if aws ecr describe-repositories --repository-names "${STACK_NAME}" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting ECR repository..."
        aws ecr delete-repository --repository-name "${STACK_NAME}" --force --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    # Also try to delete heliastes-prod if it exists
    if aws ecr describe-repositories --repository-names "heliastes-prod" --region "$AWS_REGION" >/dev/null 2>&1; then
        aws ecr delete-repository --repository-name "heliastes-prod" --force --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    log_info "ECR cleanup complete"
}

# Delete security groups
cleanup_security_groups() {
    log_step "Cleaning up security groups..."

    VPC_ID=$(aws ec2 describe-vpcs --filters "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Vpcs[0].VpcId' --output text 2>/dev/null || echo "")

    if [[ -n "$VPC_ID" ]]; then
        # Get security groups
        SG_IDS=$(aws ec2 describe-security-groups --filters "Name=vpc-id,Values=$VPC_ID" "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'SecurityGroups[*].GroupId' --output text 2>/dev/null || echo "")

        for sg_id in $SG_IDS; do
            # Skip default security group
            if aws ec2 describe-security-groups --group-ids "$sg_id" --region "$AWS_REGION" --query 'SecurityGroups[0].GroupName' --output text | grep -q "default"; then
                continue
            fi

            log_info "Deleting security group: $sg_id"
            aws ec2 delete-security-group --group-id "$sg_id" --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
    fi

    log_info "Security groups cleanup complete"
}

# Delete NAT Gateway and Elastic IP
cleanup_nat_gateway() {
    log_step "Cleaning up NAT Gateway and Elastic IP..."

    VPC_ID=$(aws ec2 describe-vpcs --filters "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Vpcs[0].VpcId' --output text 2>/dev/null || echo "")

    if [[ -n "$VPC_ID" ]]; then
        # Delete NAT Gateway
        NAT_GW_ID=$(aws ec2 describe-nat-gateways --filter "Name=vpc-id,Values=$VPC_ID" --region "$AWS_REGION" --query 'NatGateways[0].NatGatewayId' --output text 2>/dev/null || echo "")

        if [[ -n "$NAT_GW_ID" && "$NAT_GW_ID" != "None" ]]; then
            log_info "Deleting NAT Gateway..."
            aws ec2 delete-nat-gateway --nat-gateway-id "$NAT_GW_ID" --region "$AWS_REGION" >/dev/null 2>&1 || true

            # Wait for NAT Gateway to be deleted
            log_info "Waiting for NAT Gateway deletion..."
            aws ec2 wait nat-gateway-deleted --nat-gateway-ids "$NAT_GW_ID" --region "$AWS_REGION" 2>/dev/null || true
        fi

        # Delete Elastic IP
        EIP_ALLOC_ID=$(aws ec2 describe-addresses --filters "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Addresses[0].AllocationId' --output text 2>/dev/null || echo "")

        if [[ -n "$EIP_ALLOC_ID" && "$EIP_ALLOC_ID" != "None" ]]; then
            log_info "Releasing Elastic IP..."
            aws ec2 release-address --allocation-id "$EIP_ALLOC_ID" --region "$AWS_REGION" >/dev/null 2>&1 || true
        fi
    fi

    log_info "NAT Gateway cleanup complete"
}

# Delete Internet Gateway
cleanup_internet_gateway() {
    log_step "Cleaning up Internet Gateway..."

    VPC_ID=$(aws ec2 describe-vpcs --filters "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Vpcs[0].VpcId' --output text 2>/dev/null || echo "")

    if [[ -n "$VPC_ID" ]]; then
        # Detach and delete IGW
        IGW_ID=$(aws ec2 describe-internet-gateways --filters "Name=attachment.vpc-id,Values=$VPC_ID" --region "$AWS_REGION" --query 'InternetGateways[0].InternetGatewayId' --output text 2>/dev/null || echo "")

        if [[ -n "$IGW_ID" && "$IGW_ID" != "None" ]]; then
            log_info "Detaching and deleting Internet Gateway..."
            aws ec2 detach-internet-gateway --internet-gateway-id "$IGW_ID" --vpc-id "$VPC_ID" --region "$AWS_REGION" >/dev/null 2>&1 || true
            aws ec2 delete-internet-gateway --internet-gateway-id "$IGW_ID" --region "$AWS_REGION" >/dev/null 2>&1 || true
        fi
    fi

    log_info "Internet Gateway cleanup complete"
}

# Delete subnets
cleanup_subnets() {
    log_step "Cleaning up subnets..."

    VPC_ID=$(aws ec2 describe-vpcs --filters "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Vpcs[0].VpcId' --output text 2>/dev/null || echo "")

    if [[ -n "$VPC_ID" ]]; then
        # Get subnet IDs
        SUBNET_IDS=$(aws ec2 describe-subnets --filters "Name=vpc-id,Values=$VPC_ID" "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Subnets[*].SubnetId' --output text 2>/dev/null || echo "")

        for subnet_id in $SUBNET_IDS; do
            log_info "Deleting subnet: $subnet_id"
            aws ec2 delete-subnet --subnet-id "$subnet_id" --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
    fi

    log_info "Subnets cleanup complete"
}

# Delete route tables
cleanup_route_tables() {
    log_step "Cleaning up route tables..."

    VPC_ID=$(aws ec2 describe-vpcs --filters "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Vpcs[0].VpcId' --output text 2>/dev/null || echo "")

    if [[ -n "$VPC_ID" ]]; then
        # Get route table IDs (excluding main route table)
        RT_IDS=$(aws ec2 describe-route-tables --filters "Name=vpc-id,Values=$VPC_ID" "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'RouteTables[?Associations[0].Main != `true`].RouteTableId' --output text 2>/dev/null || echo "")

        for rt_id in $RT_IDS; do
            log_info "Deleting route table: $rt_id"
            aws ec2 delete-route-table --route-table-id "$rt_id" --region "$AWS_REGION" >/dev/null 2>&1 || true
        done
    fi

    log_info "Route tables cleanup complete"
}

# Delete VPC
cleanup_vpc() {
    log_step "Cleaning up VPC..."

    VPC_ID=$(aws ec2 describe-vpcs --filters "Name=tag:Application,Values=alelysee" --region "$AWS_REGION" --query 'Vpcs[0].VpcId' --output text 2>/dev/null || echo "")

    if [[ -n "$VPC_ID" ]]; then
        log_info "Deleting VPC: $VPC_ID"
        aws ec2 delete-vpc --vpc-id "$VPC_ID" --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    log_info "VPC cleanup complete"
}

# Delete CloudWatch log groups
cleanup_cloudwatch() {
    log_step "Cleaning up CloudWatch resources..."

    # Delete log group
    if aws logs describe-log-groups --log-group-name-prefix "/ecs/${STACK_NAME}" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting CloudWatch log group..."
        aws logs delete-log-group --log-group-name "/ecs/${STACK_NAME}" --region "$AWS_REGION" >/dev/null 2>&1 || true
    fi

    log_info "CloudWatch cleanup complete"
}

# Delete IAM roles and policies
cleanup_iam() {
    log_step "Cleaning up IAM resources..."

    # Delete execution role
    if aws iam get-role --role-name "${STACK_NAME}-execution-role" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting execution role..."

        # Detach managed policies
        aws iam detach-role-policy --role-name "${STACK_NAME}-execution-role" --policy-arn "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy" >/dev/null 2>&1 || true

        # Delete role
        aws iam delete-role --role-name "${STACK_NAME}-execution-role" >/dev/null 2>&1 || true
    fi

    # Delete task role
    if aws iam get-role --role-name "${STACK_NAME}-task-role" --region "$AWS_REGION" >/dev/null 2>&1; then
        log_info "Deleting task role..."

        # Detach custom policy
        aws iam detach-role-policy --role-name "${STACK_NAME}-task-role" --policy-arn "arn:aws:iam::$(aws sts get-caller-identity --query Account --output text):policy/${STACK_NAME}-s3-policy" >/dev/null 2>&1 || true

        # Delete role
        aws iam delete-role --role-name "${STACK_NAME}-task-role" >/dev/null 2>&1 || true
    fi

    # Delete custom policies
    if aws iam list-policies --scope Local --query "Policies[?PolicyName=='${STACK_NAME}-s3-policy'].Arn" --output text >/dev/null 2>&1; then
        POLICY_ARN=$(aws iam list-policies --scope Local --query "Policies[?PolicyName=='${STACK_NAME}-s3-policy'].Arn" --output text)
        aws iam delete-policy --policy-arn "$POLICY_ARN" >/dev/null 2>&1 || true
    fi

    if aws iam list-policies --scope Local --query "Policies[?PolicyName=='${STACK_NAME}-ecr-policy'].Arn" --output text >/dev/null 2>&1; then
        POLICY_ARN=$(aws iam list-policies --scope Local --query "Policies[?PolicyName=='${STACK_NAME}-ecr-policy'].Arn" --output text)
        aws iam delete-policy --policy-arn "$POLICY_ARN" >/dev/null 2>&1 || true
    fi

    log_info "IAM cleanup complete"
}

# Main cleanup function
main() {
    log_info "Starting cleanup of ${APP_NAME} resources in ${ENVIRONMENT} environment"
    log_info "Region: $AWS_REGION"
    log_info "Stack: $STACK_NAME"

    # Run cleanup in reverse dependency order
    cleanup_ecs
    cleanup_alb
    cleanup_rds
    cleanup_ecr
    cleanup_security_groups
    cleanup_nat_gateway
    cleanup_internet_gateway
    cleanup_subnets
    cleanup_route_tables
    cleanup_vpc
    cleanup_cloudwatch
    cleanup_iam

    log_info ""
    log_info "🎉 Cleanup Complete!"
    log_info ""
    log_info "📋 Summary:"
    log_info "   Environment: ${ENVIRONMENT}"
    log_info "   Region: ${AWS_REGION}"
    log_info "   Stack: ${STACK_NAME}"
    log_info ""
    log_info "✅ All resources have been scheduled for deletion"
    log_info "   Some resources may take a few minutes to be fully removed"
}

# Run main function
get_region
main "$@"

