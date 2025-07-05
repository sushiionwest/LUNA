# Setup Code Signing Environment
# This script sets up the Azure Key Vault integration for code signing

param(
    [Parameter(Mandatory=$true)]
    [string]$KeyVaultName,
    
    [Parameter(Mandatory=$true)]
    [string]$CertificateName,
    
    [Parameter(Mandatory=$false)]
    [string]$ResourceGroupName = "luna-signing-rg",
    
    [Parameter(Mandatory=$false)]
    [string]$Location = "East US 2"
)

Write-Host "🔐 Setting up code signing environment..." -ForegroundColor Green

# Check if Azure CLI is installed
if (-not (Get-Command az -ErrorAction SilentlyContinue)) {
    Write-Error "Azure CLI is not installed. Please install it first: https://docs.microsoft.com/en-us/cli/azure/install-azure-cli"
    exit 1
}

# Login check
try {
    $account = az account show 2>$null | ConvertFrom-Json
    if (-not $account) {
        throw "Not logged in"
    }
    Write-Host "✅ Logged in as: $($account.user.name)" -ForegroundColor Green
} catch {
    Write-Host "❌ Please login to Azure CLI first:" -ForegroundColor Red
    Write-Host "   az login"
    exit 1
}

# Create Resource Group
Write-Host "📦 Creating resource group: $ResourceGroupName"
try {
    az group create --name $ResourceGroupName --location $Location --output none
    Write-Host "✅ Resource group created successfully" -ForegroundColor Green
} catch {
    Write-Host "⚠️  Resource group might already exist, continuing..." -ForegroundColor Yellow
}

# Create Key Vault
Write-Host "🔑 Creating Key Vault: $KeyVaultName"
try {
    az keyvault create `
        --name $KeyVaultName `
        --resource-group $ResourceGroupName `
        --location $Location `
        --enabled-for-disk-encryption true `
        --enabled-for-deployment true `
        --enabled-for-template-deployment true `
        --output none
    
    Write-Host "✅ Key Vault created successfully" -ForegroundColor Green
} catch {
    Write-Host "⚠️  Key Vault might already exist, continuing..." -ForegroundColor Yellow
}

# Set Key Vault permissions for the current user
Write-Host "🔓 Setting Key Vault permissions..."
$currentUser = az ad signed-in-user show --query userPrincipalName -o tsv
az keyvault set-policy `
    --name $KeyVaultName `
    --upn $currentUser `
    --secret-permissions get list set delete recover backup restore `
    --certificate-permissions get list create delete import recover backup restore managecontacts manageissuers getissuers listissuers setissuers deleteissuers `
    --key-permissions get list create delete import recover backup restore `
    --output none

Write-Host "✅ Permissions set for: $currentUser" -ForegroundColor Green

# Create self-signed certificate for development/testing
Write-Host "📜 Creating development certificate..."
$certPolicy = @"
{
  "issuerParameters": {
    "name": "Self"
  },
  "keyProperties": {
    "exportable": true,
    "keySize": 2048,
    "keyType": "RSA",
    "reuseKey": false
  },
  "secretProperties": {
    "contentType": "application/x-pkcs12"
  },
  "x509CertificateProperties": {
    "subject": "CN=Luna Development Certificate",
    "subjectAlternativeNames": {
      "dnsNames": ["luna-agent.local"]
    },
    "validityInMonths": 12,
    "keyUsage": ["cRLSign", "dataEncipherment", "digitalSignature", "keyEncipherment", "keyAgreement", "keyCertSign"],
    "ekus": ["1.3.6.1.5.5.7.3.3"]
  },
  "lifetimeActions": [
    {
      "trigger": {
        "lifetimePercentage": 80
      },
      "action": {
        "actionType": "AutoRenew"
      }
    }
  ]
}
"@

$certPolicy | Out-File -FilePath "cert-policy.json" -Encoding UTF8

try {
    az keyvault certificate create `
        --vault-name $KeyVaultName `
        --name $CertificateName `
        --policy "@cert-policy.json" `
        --output none
    
    Write-Host "✅ Development certificate created" -ForegroundColor Green
    Remove-Item "cert-policy.json" -Force
} catch {
    Write-Host "⚠️  Certificate might already exist, continuing..." -ForegroundColor Yellow
    Remove-Item "cert-policy.json" -Force -ErrorAction SilentlyContinue
}

# Create service principal for GitHub Actions
Write-Host "🤖 Creating service principal for GitHub Actions..."
$spName = "luna-signing-sp"
$subscriptionId = az account show --query id -o tsv

try {
    $sp = az ad sp create-for-rbac `
        --name $spName `
        --role "Key Vault Secrets User" `
        --scopes "/subscriptions/$subscriptionId/resourceGroups/$ResourceGroupName/providers/Microsoft.KeyVault/vaults/$KeyVaultName" `
        --sdk-auth | ConvertFrom-Json
    
    # Set additional Key Vault permissions for the service principal
    az keyvault set-policy `
        --name $KeyVaultName `
        --spn $sp.clientId `
        --secret-permissions get list `
        --certificate-permissions get list `
        --key-permissions get list `
        --output none
    
    Write-Host "✅ Service principal created successfully" -ForegroundColor Green
    Write-Host ""
    Write-Host "🔧 GitHub Secrets Configuration:" -ForegroundColor Cyan
    Write-Host "Add these secrets to your GitHub repository:"
    Write-Host ""
    Write-Host "AZURE_CREDENTIALS:" -ForegroundColor Yellow
    $sp | ConvertTo-Json -Depth 10
    Write-Host ""
    Write-Host "AZURE_KEYVAULT_NAME:" -ForegroundColor Yellow
    Write-Host $KeyVaultName
    Write-Host ""
    Write-Host "CERT_PASSWORD:" -ForegroundColor Yellow
    Write-Host "(Set this to a secure password for certificate export)"
    Write-Host ""
    
} catch {
    Write-Host "❌ Failed to create service principal. You may need to do this manually." -ForegroundColor Red
    Write-Host "Run: az ad sp create-for-rbac --name $spName --role 'Key Vault Secrets User' --scopes '/subscriptions/$subscriptionId/resourceGroups/$ResourceGroupName/providers/Microsoft.KeyVault/vaults/$KeyVaultName' --sdk-auth"
}

# Create GitHub Actions workflow secrets template
Write-Host "📋 Creating secrets template for GitHub Actions..."
$secretsTemplate = @"
# GitHub Repository Secrets Configuration
# Add these secrets to your GitHub repository settings:

## Required Secrets:

### AZURE_CREDENTIALS
Paste the JSON output from the service principal creation above.

### AZURE_KEYVAULT_NAME
$KeyVaultName

### CERT_PASSWORD
A secure password for certificate export (generate a strong password)

## Optional Secrets:

### SQUIRREL_UPDATE_URL
https://github.com/YOUR_USERNAME/luna/releases/latest/download

### UPDATE_SERVER_URL
https://api.github.com/repos/YOUR_USERNAME/luna/releases/latest

## Setup Instructions:

1. Go to your GitHub repository
2. Navigate to Settings > Secrets and variables > Actions
3. Add each secret listed above
4. Update the repository URL in the workflow files
5. Commit and push to trigger the build

## Production Certificate Setup:

For production releases, consider purchasing a proper code signing certificate from:
- DigiCert
- Sectigo (formerly Comodo)
- GlobalSign

These certificates provide better trust indicators for end users.

## Alternative: Sigstore (Free, Open Source)

For open source projects, consider using Sigstore for keyless signing:
https://docs.sigstore.dev/
"@

$secretsTemplate | Out-File -FilePath "github-secrets-setup.md" -Encoding UTF8

Write-Host "✅ Setup complete!" -ForegroundColor Green
Write-Host ""
Write-Host "📝 Next steps:" -ForegroundColor Cyan
Write-Host "1. Review the output above for GitHub secrets configuration"
Write-Host "2. Add the secrets to your GitHub repository"
Write-Host "3. Review 'github-secrets-setup.md' for detailed instructions"
Write-Host "4. Test the build pipeline with a push to the repository"
Write-Host ""
Write-Host "🔐 Security Notes:" -ForegroundColor Yellow
Write-Host "- The development certificate is self-signed and should only be used for testing"
Write-Host "- For production, obtain a proper code signing certificate"
Write-Host "- Keep the CERT_PASSWORD secret secure and rotate it regularly"
Write-Host "- Monitor Key Vault access logs for any suspicious activity"