package rds_test

import (
	"testing"

	"github.com/gruntwork-io/terratest/modules/terraform"
	"github.com/stretchr/testify/assert"
)

// TestRDSBackupConfiguration verifies backup_retention_period, backup_window,
// and deletion_protection are explicitly set in the RDS module.
func TestRDSBackupConfiguration(t *testing.T) {
	t.Parallel()

	opts := &terraform.Options{
		TerraformDir: ".",
		Vars: map[string]interface{}{
			"environment":        "prod",
			"vpc_id":             "vpc-00000000",
			"private_subnet_ids": []string{"subnet-00000001", "subnet-00000002"},
			"db_name":            "predictiq",
			"db_username":        "admin",
			"db_password":        "testpassword123",
			"db_instance_class":  "db.t3.micro",
			"allocated_storage":  20,
			"backup_retention":   7,
			"deletion_protection": true,
		},
		// Plan only — no real AWS resources are created
		PlanFilePath: "/tmp/rds-test.tfplan",
	}

	plan := terraform.InitAndPlanAndShowWithStruct(t, opts)

	rds := plan.ResourcePlannedValuesMap["aws_db_instance.main"]
	assert.NotNil(t, rds, "aws_db_instance.main must be planned")

	values := rds.AttributeValues

	// backup_retention_period must be >= 7
	retention, ok := values["backup_retention_period"].(float64)
	assert.True(t, ok, "backup_retention_period must be a number")
	assert.GreaterOrEqual(t, int(retention), 7, "backup_retention_period must be at least 7 days")

	// backup_window must be set
	backupWindow, ok := values["backup_window"].(string)
	assert.True(t, ok, "backup_window must be a string")
	assert.NotEmpty(t, backupWindow, "backup_window must not be empty")

	// deletion_protection must be true for prod
	deletionProtection, ok := values["deletion_protection"].(bool)
	assert.True(t, ok, "deletion_protection must be a boolean")
	assert.True(t, deletionProtection, "deletion_protection must be true for production")
}
