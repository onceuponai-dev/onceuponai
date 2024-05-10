variable "location" {
  type        = string
  description = "The location where all resources will be created."
}

variable "resource_group_name" {
  type        = string
  description = "The name of the resource group."
}

variable "lancedb_storage" {
  type = any
}

variable "tags" {
  type = any
}