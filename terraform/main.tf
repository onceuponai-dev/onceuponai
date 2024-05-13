terraform {
  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 3.0"
    }
  }

  backend "azurerm" {}
}

provider "azurerm" {
  features {}
}

resource "azurerm_resource_group" "rg" {
  name     = var.resource_group_name
  location = var.location
}

# LANCEDB

resource "azurerm_storage_account" "lancedb" {
  name                     = var.lancedb_storage.account_name
  resource_group_name      = azurerm_resource_group.rg.name
  location                 = azurerm_resource_group.rg.location
  account_tier             = "Standard"
  account_replication_type = "LRS"

  tags = {
    environment = "staging"
  }
}

resource "azurerm_storage_container" "lancedb" {
  name                  = var.lancedb_storage.container_name
  storage_account_name  = azurerm_storage_account.lancedb.name
  container_access_type = "private"
}

# BOT SERVICE

resource "azurerm_bot_channels_registration" "bot" {
  name                = var.bot.name
  location            = "global"
  resource_group_name = azurerm_resource_group.rg.name
  sku                 = var.bot.sku
  microsoft_app_id    = var.bot.microsoft_app_id
}

resource "azurerm_bot_channel_ms_teams" "bot" {
  bot_name            = azurerm_bot_channels_registration.bot.name
  location            = azurerm_bot_channels_registration.bot.location
  resource_group_name = azurerm_resource_group.rg.name
}