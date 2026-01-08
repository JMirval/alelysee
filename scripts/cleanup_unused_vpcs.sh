#!/usr/bin/env bash
set -euo pipefail

# Clean up unused VPCs script
# This script safely removes VPCs that have no subnets or security groups

echo "Scanning all VPCs..."

unused_vpcs=""

for vpc_id in $(aws ec2 describe-vpcs --query 'Vpcs[*].VpcId' --output text); do
    # Skip default VPC
    is_default=$(aws ec2 describe-vpcs --vpc-ids "$vpc_id" --query 'Vpcs[0].IsDefault' --output text 2>/dev/null)
    if [[ "$is_default" == "True" ]]; then
        continue
    fi

    # Check for any subnets
    subnet_count=$(aws ec2 describe-subnets --filters "Name=vpc-id,Values=$vpc_id" --query 'Subnets[*].SubnetId' --output text 2>/dev/null | wc -l)
    if [ "$subnet_count" -gt 0 ]; then
        continue
    fi

    # Check for any security groups (excluding default)
    sg_count=$(aws ec2 describe-security-groups --filters "Name=vpc-id,Values=$vpc_id" --query 'SecurityGroups[?GroupName!=`default`].GroupId' --output text 2>/dev/null | wc -l)
    if [ "$sg_count" -gt 0 ]; then
        continue
    fi

    # If we get here, VPC is truly empty
    vpc_name=$(aws ec2 describe-vpcs --vpc-ids "$vpc_id" --query 'Vpcs[0].Tags[?Key==`Name`].Value|[0]' --output text 2>/dev/null || echo "unnamed")
    if [[ "$vpc_name" == "None" ]]; then
        vpc_name="unnamed"
    fi

    echo "Found truly empty VPC: $vpc_id ($vpc_name)"
    unused_vpcs="$unused_vpcs $vpc_id"
done

if [[ -n "$unused_vpcs" ]]; then
    echo ""
    echo "🗑️  Deleting empty VPCs..."
    for vpc_id in $unused_vpcs; do
        echo "Deleting VPC: $vpc_id"
        if aws ec2 delete-vpc --vpc-id "$vpc_id" 2>/dev/null; then
            echo "✅ Successfully deleted VPC: $vpc_id"
        else
            echo "❌ Failed to delete VPC: $vpc_id"
        fi
    done
    echo ""
    echo "✅ Cleanup complete!"
else
    echo "No truly empty VPCs found."
    echo "Note: All VPCs have subnets or security groups attached."
    echo "Use 'make aws-cleanup-force' to delete everything (DANGER!)."
fi

