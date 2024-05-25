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

# CONTAINER

resource "azurerm_storage_share" "huggingface" {
  name                 = "huggingface"
  storage_account_name  = azurerm_storage_account.lancedb.name
  quota                = 10
}

resource "azurerm_container_group" "example" {
  name                = "example-containergroup"
  resource_group_name      = azurerm_resource_group.rg.name
  location                 = azurerm_resource_group.rg.location
  os_type             = "Linux"
  ip_address_type     = "Public"

  container {
    name   = "hello-world"
    image  = "caddy:2.6"
    cpu    = "0.5"
    memory = "1.5"

    ports {
      port     = 443
      protocol = "TCP"
    }
  }

  container {
    name   = "bot"
    image  = "docker.io/qooba/cuda:mstech2024"
    cpu    = "1"
    memory = "1.5"

    volume {
      name = "huggingface"
      mount_path = "/home/jovyan/.cache/huggingface"
      share_name          = azurerm_storage_share.huggingface.name
      storage_account_name = azurerm_storage_account.lancedb.name
      storage_account_key = azurerm_storage_account.lancedb.primary_access_key
    }

    # gpu {
    #   count = 1
    #   sku   = "K80"
    # }
  }


  tags = {
    environment = "staging"
  }
}
