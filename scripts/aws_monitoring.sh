#!/usr/bin/env bash
set -euo pipefail

# Heliastes AWS Monitoring and Analytics Setup
# ============================================
#
# This script sets up comprehensive monitoring and analytics for the Heliastes application:
# - CloudWatch Logs for centralized logging
# - CloudWatch Metrics and Dashboards for monitoring
# - Route 53 Health Checks for endpoint monitoring
# - CloudWatch Alarms for alerting
# - X-Ray for application tracing (optional)
#
# Prerequisites:
# - AWS CLI configured
# - Application deployed with ALB
# - jq installed

ENVIRONMENT="${1:-dev}"
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

# Load environment variables
if [[ -f ".env" ]]; then
    log_info "Loading environment from .env"
    set -a
    source .env
    set +a
else
    log_error ".env file not found"
    exit 1
fi

# Get AWS account and region
get_aws_info() {
    AWS_ACCOUNT_ID="$(aws sts get-caller-identity --query Account --output text)"
    AWS_REGION="${AWS_REGION:-$(aws configure get region)}"

    if [[ -z "$AWS_REGION" ]]; then
        log_error "AWS_REGION not set. Please configure AWS CLI or set AWS_REGION."
        exit 1
    fi

    log_info "Using AWS Account: $AWS_ACCOUNT_ID"
    log_info "Using AWS Region: $AWS_REGION"
    log_info "Environment: $ENVIRONMENT"
    log_info "Stack Name: $STACK_NAME"
}

# Create CloudWatch Logs Group
create_cloudwatch_logs() {
    log_step "Creating CloudWatch Logs Group..."

    aws logs create-log-group \
        --log-group-name "/aws/ecs/${STACK_NAME}" \
        --tags "Environment=${ENVIRONMENT},Application=${APP_NAME}" \
        2>/dev/null || log_warn "Log group may already exist"

    log_info "CloudWatch Logs Group: /aws/ecs/${STACK_NAME}"
}

# Create CloudWatch Dashboard
create_cloudwatch_dashboard() {
    log_step "Creating CloudWatch Dashboard..."

    DASHBOARD_BODY=$(cat <<EOF
{
    "widgets": [
        {
            "type": "metric",
            "x": 0,
            "y": 0,
            "width": 12,
            "height": 6,
            "properties": {
                "metrics": [
                    ["AWS/ECS", "CPUUtilization", "ServiceName", "${STACK_NAME}-service", "ClusterName", "${STACK_NAME}-cluster"],
                    [".", "MemoryUtilization", ".", ".", ".", "."]
                ],
                "view": "timeSeries",
                "stacked": false,
                "region": "${AWS_REGION}",
                "title": "ECS Resource Utilization",
                "period": 300
            }
        },
        {
            "type": "metric",
            "x": 12,
            "y": 0,
            "width": 12,
            "height": 6,
            "properties": {
                "metrics": [
                    ["AWS/ApplicationELB", "RequestCount", "LoadBalancer", "${ALB_NAME}", {"stat": "Sum"}],
                    [".", "TargetResponseTime", ".", ".", {"stat": "Average"}],
                    [".", "HTTPCode_Target_2XX_Count", ".", ".", {"stat": "Sum"}],
                    [".", "HTTPCode_Target_4XX_Count", ".", ".", {"stat": "Sum"}],
                    [".", "HTTPCode_Target_5XX_Count", ".", ".", {"stat": "Sum"}]
                ],
                "view": "timeSeries",
                "stacked": false,
                "region": "${AWS_REGION}",
                "title": "ALB Metrics",
                "period": 300
            }
        },
        {
            "type": "log",
            "x": 0,
            "y": 6,
            "width": 24,
            "height": 6,
            "properties": {
                "query": "SOURCE '/aws/ecs/${STACK_NAME}' | fields @timestamp, @message | sort @timestamp desc | limit 100",
                "region": "${AWS_REGION}",
                "title": "Application Logs",
                "view": "table"
            }
        },
        {
            "type": "metric",
            "x": 0,
            "y": 12,
            "width": 12,
            "height": 6,
            "properties": {
                "metrics": [
                    ["AWS/RDS", "DatabaseConnections", "DBInstanceIdentifier", "${STACK_NAME}-db"],
                    [".", "ReadIOPS", ".", "."],
                    [".", "WriteIOPS", ".", "."]
                ],
                "view": "timeSeries",
                "stacked": false,
                "region": "${AWS_REGION}",
                "title": "RDS Metrics",
                "period": 300
            }
        },
        {
            "type": "metric",
            "x": 12,
            "y": 12,
            "width": 12,
            "height": 6,
            "properties": {
                "metrics": [
                    ["AWS/S3", "BucketSizeBytes", "BucketName", "${S3_BUCKET}", "StorageType", "StandardStorage"],
                    [".", "NumberOfObjects", ".", ".", ".", "."]
                ],
                "view": "timeSeries",
                "stacked": false,
                "region": "${AWS_REGION}",
                "title": "S3 Storage Metrics",
                "period": 3600
            }
        }
    ]
}
EOF
)

    aws cloudwatch put-dashboard \
        --dashboard-name "${STACK_NAME}-dashboard" \
        --dashboard-body "$DASHBOARD_BODY"

    log_info "CloudWatch Dashboard: ${STACK_NAME}-dashboard"
    log_info "View at: https://${AWS_REGION}.console.aws.amazon.com/cloudwatch/home?region=${AWS_REGION}#dashboards:name=${STACK_NAME}-dashboard"
}

# Create Route 53 Health Checks
create_route53_health_checks() {
    log_step "Creating Route 53 Health Checks..."

    # ALB_DNS should already be set by get_alb_info()
    if [[ -z "$ALB_DNS" ]]; then
        log_error "ALB DNS not available. Cannot create health checks."
        return 1
    fi

    log_info "Creating health check for: http://${ALB_DNS}/api/health"

    # Create health check for main application
    HC_ID=$(aws route53 create-health-check \
        --caller-reference "${STACK_NAME}-health-check-$(date +%s)" \
        --health-check-config "{
            \"IPAddress\": null,
            \"Port\": 80,
            \"Type\": \"HTTP\",
            \"ResourcePath\": \"/api/health\",
            \"FullyQualifiedDomainName\": \"${ALB_DNS}\",
            \"RequestInterval\": 30,
            \"FailureThreshold\": 3,
            \"MeasureLatency\": true,
            \"EnableSNI\": true
        }" \
        --query 'HealthCheck.Id' \
        --output text)

    aws route53 change-tags-for-resource \
        --resource-type healthcheck \
        --resource-id "$HC_ID" \
        --add-tags "Key=Name,Value=${STACK_NAME}-health-check" "Key=Environment,Value=${ENVIRONMENT}" "Key=Application,Value=${APP_NAME}"

    log_info "Route 53 Health Check created: $HC_ID"
    log_info "Monitor at: https://${AWS_REGION}.console.aws.amazon.com/route53/healthchecks/home"
}

# Create CloudWatch Alarms
create_cloudwatch_alarms() {
    log_step "Creating CloudWatch Alarms..."

    # ALB 5xx errors alarm
    if [[ -n "$ALB_NAME" ]]; then
        aws cloudwatch put-metric-alarm \
            --alarm-name "${STACK_NAME}-alb-5xx-errors" \
            --alarm-description "ALB 5xx errors > 5% for 5 minutes" \
            --metric-name HTTPCode_Target_5XX_Count \
            --namespace AWS/ApplicationELB \
            --statistic Sum \
            --period 300 \
            --threshold 5 \
            --comparison-operator GreaterThanThreshold \
            --evaluation-periods 1 \
            --dimensions "Name=LoadBalancer,Value=${ALB_NAME}" \
            --alarm-actions "arn:aws:sns:${AWS_REGION}:${AWS_ACCOUNT_ID}:${STACK_NAME}-alerts"
        log_info "Created ALB 5xx errors alarm"
    else
        log_warn "ALB name not available, skipping ALB alarms"
    fi

    # ECS CPU utilization alarm
    aws cloudwatch put-metric-alarm \
        --alarm-name "${STACK_NAME}-ecs-high-cpu" \
        --alarm-description "ECS CPU utilization > 80% for 10 minutes" \
        --metric-name CPUUtilization \
        --namespace AWS/ECS \
        --statistic Average \
        --period 300 \
        --threshold 80 \
        --comparison-operator GreaterThanThreshold \
        --evaluation-periods 2 \
        --dimensions "Name=ServiceName,Value=${STACK_NAME}-service" "Name=ClusterName,Value=${STACK_NAME}-cluster" \
        --alarm-actions "arn:aws:sns:${AWS_REGION}:${AWS_ACCOUNT_ID}:${STACK_NAME}-alerts" \
        2>/dev/null || log_warn "ECS CPU alarm creation failed (ECS may not exist yet)"

    # RDS connection alarm
    aws cloudwatch put-metric-alarm \
        --alarm-name "${STACK_NAME}-rds-connections" \
        --alarm-description "RDS database connections > 80 for 5 minutes" \
        --metric-name DatabaseConnections \
        --namespace AWS/RDS \
        --statistic Maximum \
        --period 300 \
        --threshold 80 \
        --comparison-operator GreaterThanThreshold \
        --evaluation-periods 1 \
        --dimensions "Name=DBInstanceIdentifier,Value=${STACK_NAME}-db" \
        --alarm-actions "arn:aws:sns:${AWS_REGION}:${AWS_ACCOUNT_ID}:${STACK_NAME}-alerts" \
        2>/dev/null || log_warn "RDS alarm creation failed (RDS may not exist yet)"

    log_info "CloudWatch Alarms created"
}

# Create SNS Topic for alerts (if it doesn't exist)
create_sns_topic() {
    log_step "Creating SNS Topic for alerts..."

    TOPIC_ARN=$(aws sns create-topic \
        --name "${STACK_NAME}-alerts" \
        --tags "Key=Environment,Value=${ENVIRONMENT}" "Key=Application,Value=${APP_NAME}" \
        --query 'TopicArn' \
        --output text 2>/dev/null || aws sns list-topics --query "Topics[?contains(TopicArn, '${STACK_NAME}-alerts')].TopicArn" --output text)

    if [[ -n "$TOPIC_ARN" ]]; then
        log_info "SNS Topic: $TOPIC_ARN"

        # Subscribe email (optional - requires manual confirmation)
        read -p "Enter email address for alerts (leave empty to skip): " ALERT_EMAIL
        if [[ -n "$ALERT_EMAIL" ]]; then
            aws sns subscribe \
                --topic-arn "$TOPIC_ARN" \
                --protocol email \
                --notification-endpoint "$ALERT_EMAIL"
            log_info "Email subscription created. Check your email to confirm."
        fi
    else
        log_warn "SNS Topic creation failed"
    fi
}

# Setup X-Ray (optional)
setup_xray() {
    log_step "Setting up AWS X-Ray..."

    # Create X-Ray group
    aws xray create-group \
        --group-name "${STACK_NAME}" \
        --filter-expression "service(\"${APP_NAME}\")" \
        --tags "Key=Environment,Value=${ENVIRONMENT}" "Key=Application,Value=${APP_NAME}" \
        2>/dev/null || log_warn "X-Ray group may already exist"

    log_info "X-Ray Group: ${STACK_NAME}"
    log_info "View traces at: https://${AWS_REGION}.console.aws.amazon.com/xray/home"
}

# Get ALB information for metrics and health checks
get_alb_info() {
    log_step "Retrieving ALB information..."

    # Try to get ALB by name
    ALB_INFO=$(aws elbv2 describe-load-balancers \
        --names "${STACK_NAME}-alb" \
        2>/dev/null || echo "")

    if [[ -z "$ALB_INFO" ]]; then
        log_error "ALB '${STACK_NAME}-alb' not found!"
        log_error "Make sure the application is deployed before setting up monitoring."
        log_error "Run deployment first: make deploy-${ENVIRONMENT}"
        exit 1
    fi

    ALB_ARN=$(echo "$ALB_INFO" | jq -r '.LoadBalancers[0].LoadBalancerArn')
    ALB_DNS=$(echo "$ALB_INFO" | jq -r '.LoadBalancers[0].DNSName')
    ALB_NAME=$(echo "$ALB_INFO" | jq -r '.LoadBalancers[0].LoadBalancerName')

    log_info "Found ALB: $ALB_NAME"
    log_info "ALB ARN: $ALB_ARN"
    log_info "ALB DNS: $ALB_DNS"

    # Verify ALB is accessible and health endpoint responds
    log_info "Testing health endpoint: http://${ALB_DNS}/api/health"
    if ! curl -f --max-time 30 --retry 3 --retry-delay 5 "http://${ALB_DNS}/api/health" >/dev/null 2>&1; then
        log_error "❌ ALB health check failed! Cannot proceed with monitoring setup."
        log_error "The deployed application is not responding on the health endpoint."
        log_error ""
        log_error "Troubleshooting steps:"
        log_error "1. Check if the application is deployed: make aws-status"
        log_error "2. Check ECS service status in AWS console"
        log_error "3. Check application logs: make logs"
        log_error "4. Verify the health endpoint manually: curl http://${ALB_DNS}/api/health"
        exit 1
    else
        log_info "✅ ALB health check passed - application is responding"
    fi
}

# Main function
main() {
    log_info "Setting up AWS monitoring and analytics for ${APP_NAME}"
    log_info "Environment: ${ENVIRONMENT}"
    log_info ""
    log_warn "⚠️  IMPORTANT: This script sets up monitoring for DEPLOYED applications only."
    log_warn "   Make sure your application is deployed and the ALB is accessible before running this."
    log_warn "   Run 'make deploy-${ENVIRONMENT}' first if you haven't deployed yet."
    log_info ""

    get_aws_info
    get_alb_info

    create_cloudwatch_logs
    create_cloudwatch_dashboard
    create_route53_health_checks
    create_sns_topic
    create_cloudwatch_alarms
    setup_xray

    log_info ""
    log_info "🎉 Monitoring and analytics setup complete!"
    log_info ""
    log_info "📊 Resources Created:"
    log_info "   Dashboard: https://${AWS_REGION}.console.aws.amazon.com/cloudwatch/home?region=${AWS_REGION}#dashboards:name=${STACK_NAME}-dashboard"
    log_info "   Health Checks: https://${AWS_REGION}.console.aws.amazon.com/route53/healthchecks/home"
    log_info "   Alarms: https://${AWS_REGION}.console.aws.amazon.com/cloudwatch/home?region=${AWS_REGION}#alarmsV2:"
    log_info "   X-Ray: https://${AWS_REGION}.console.aws.amazon.com/xray/home"
    log_info "   Logs: https://${AWS_REGION}.console.aws.amazon.com/cloudwatch/home?region=${AWS_REGION}#logsV2:log-groups/log-group/%2Faws%2Fecs%2F${STACK_NAME}"
    log_info ""
    log_info "🌐 Application Health Endpoints:"
    log_info "   Health Check: http://${ALB_DNS}/api/health"
    log_info "   Detailed Health: http://${ALB_DNS}/api/health/detailed"
    log_info "   Metrics: http://${ALB_DNS}/api/metrics"
    log_info ""
    log_info "✅ Health checks are monitoring your DEPLOYED application, not local development."
    log_info "   Route 53 health checks run every 30 seconds against: http://${ALB_DNS}/api/health"
}

# Run main function
main "$@"
