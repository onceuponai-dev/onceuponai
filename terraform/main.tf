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

# VM

resource "azurerm_resource_group" "rg_vm" {
  name     = "${var.resource_group_name}_VM"
  location = var.location
}


resource "azurerm_virtual_network" "vnet" {
  name                = "myVNet"
  address_space       = ["10.0.0.0/16"]
  location            = azurerm_resource_group.rg_vm.location
  resource_group_name = azurerm_resource_group.rg_vm.name
}

resource "azurerm_subnet" "subnet" {
  name                 = "mySubnet"
  resource_group_name  = azurerm_resource_group.rg_vm.name
  virtual_network_name = azurerm_virtual_network.vnet.name
  address_prefixes     = ["10.0.1.0/24"]
}

resource "azurerm_network_interface" "nic" {
  name                = "myNIC"
  location            = azurerm_resource_group.rg_vm.location
  resource_group_name = azurerm_resource_group.rg_vm.name

  ip_configuration {
    name                          = "myNICConfig"
    subnet_id                     = azurerm_subnet.subnet.id
    private_ip_address_allocation = "Dynamic"
    public_ip_address_id          = azurerm_public_ip.public_ip.id
  }
}

resource "azurerm_public_ip" "public_ip" {
  name                = "myPublicIP"
  location            = azurerm_resource_group.rg_vm.location
  resource_group_name = azurerm_resource_group.rg_vm.name
  allocation_method   = "Dynamic"
}

resource "azurerm_network_security_group" "nsg" {
  name                = "myNSG"
  location            = azurerm_resource_group.rg_vm.location
  resource_group_name = azurerm_resource_group.rg_vm.name
}

resource "azurerm_network_security_rule" "nsr" {
  name                        = "SSH"
  priority                    = 1001
  direction                   = "Inbound"
  access                      = "Allow"
  protocol                    = "Tcp"
  source_port_range           = "*"
  destination_port_range      = "22"
  source_address_prefix       = "*"
  destination_address_prefix  = "*"
  resource_group_name         = azurerm_resource_group.rg_vm.name
  network_security_group_name = azurerm_network_security_group.nsg.name
}

resource "azurerm_linux_virtual_machine" "vm" {
  name                = "myVM"
  resource_group_name = azurerm_resource_group.rg_vm.name
  location            = azurerm_resource_group.rg_vm.location
  size                = "Standard_NC4as_T4_v3"
  admin_username      = "qba"
  network_interface_ids = [
    azurerm_network_interface.nic.id,
  ]

  os_disk {
    caching              = "ReadWrite"
    storage_account_type = "StandardSSD_LRS"
  }

  source_image_reference {
    #publisher = "microsoft-dsvm"
    #offer     = "ubuntu-1804"
    #sku       = "1804-gen2"
    #version   = "latest"

    publisher = "Canonical"
    offer     = "0001-com-ubuntu-server-jammy"
    sku       = "22_04-lts"
    version   = "latest"
  }

  computer_name                   = "qooba-gpu-spot"
  admin_ssh_key {
    username   = "qba"
    public_key = file("./ssh/id_rsa.pub")
  }

  priority = "Spot"
  eviction_policy = "Deallocate"

}

output "vm_public_ip_address" {
  value = azurerm_public_ip.public_ip.ip_address
  description = "The public IP address of the VM."
}


