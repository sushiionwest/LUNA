using System;
using System.Text.Json.Serialization;

namespace LunaBrokerService
{
    // Request/Response Models
    public class BrokerRequest
    {
        [JsonPropertyName("requestId")]
        public string RequestId { get; set; } = string.Empty;

        [JsonPropertyName("operation")]
        public string Operation { get; set; } = string.Empty;

        [JsonPropertyName("parameters")]
        public object? Parameters { get; set; }

        [JsonPropertyName("timestamp")]
        public DateTime Timestamp { get; set; } = DateTime.UtcNow;

        [JsonPropertyName("signature")]
        public string? Signature { get; set; }
    }

    public class BrokerResponse
    {
        [JsonPropertyName("success")]
        public bool Success { get; set; }

        [JsonPropertyName("data")]
        public object? Data { get; set; }

        [JsonPropertyName("errorMessage")]
        public string? ErrorMessage { get; set; }

        [JsonPropertyName("timestamp")]
        public DateTime Timestamp { get; set; } = DateTime.UtcNow;
    }

    // Parameter Models for different operations
    public class UIAutomationClickParameters
    {
        [JsonPropertyName("x")]
        public int X { get; set; }

        [JsonPropertyName("y")]
        public int Y { get; set; }

        [JsonPropertyName("button")]
        public string Button { get; set; } = "left"; // left, right, middle
    }

    public class UIAutomationSendKeysParameters
    {
        [JsonPropertyName("keys")]
        public string Keys { get; set; } = string.Empty;

        [JsonPropertyName("targetWindow")]
        public string? TargetWindow { get; set; }
    }

    public class RegistryParameters
    {
        [JsonPropertyName("keyPath")]
        public string KeyPath { get; set; } = string.Empty;

        [JsonPropertyName("valueName")]
        public string ValueName { get; set; } = string.Empty;

        [JsonPropertyName("value")]
        public object? Value { get; set; }
    }

    public class ProcessParameters
    {
        [JsonPropertyName("fileName")]
        public string FileName { get; set; } = string.Empty;

        [JsonPropertyName("arguments")]
        public string? Arguments { get; set; }

        [JsonPropertyName("processId")]
        public int ProcessId { get; set; }

        [JsonPropertyName("workingDirectory")]
        public string? WorkingDirectory { get; set; }
    }

    public class FileParameters
    {
        [JsonPropertyName("filePath")]
        public string FilePath { get; set; } = string.Empty;

        [JsonPropertyName("content")]
        public string? Content { get; set; }

        [JsonPropertyName("encoding")]
        public string Encoding { get; set; } = "utf-8";
    }

    // Window information model
    public class WindowInfo
    {
        [JsonPropertyName("handle")]
        public IntPtr Handle { get; set; }

        [JsonPropertyName("title")]
        public string Title { get; set; } = string.Empty;

        [JsonPropertyName("className")]
        public string ClassName { get; set; } = string.Empty;

        [JsonPropertyName("processId")]
        public int ProcessId { get; set; }

        [JsonPropertyName("processName")]
        public string ProcessName { get; set; } = string.Empty;

        [JsonPropertyName("isVisible")]
        public bool IsVisible { get; set; }

        [JsonPropertyName("bounds")]
        public WindowBounds Bounds { get; set; } = new();
    }

    public class WindowBounds
    {
        [JsonPropertyName("x")]
        public int X { get; set; }

        [JsonPropertyName("y")]
        public int Y { get; set; }

        [JsonPropertyName("width")]
        public int Width { get; set; }

        [JsonPropertyName("height")]
        public int Height { get; set; }
    }

    // Security exception for broker operations
    public class SecurityException : Exception
    {
        public SecurityException(string message) : base(message) { }
        public SecurityException(string message, Exception innerException) : base(message, innerException) { }
    }
}