use e2b::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== E2B Code Interpreter Demo ===");
    println!("This example demonstrates multi-language code execution using the code-interpreter-v1 template.\n");

    let client = Client::new()?;

    println!("Creating sandbox with code interpreter...");
    let sandbox = client
        .sandbox()
        .template("code-interpreter-v1")  // Required for multi-language support
        .metadata(json!({
            "example": "code_interpreter",
            "languages": "python,javascript"
        }))
        .timeout(300)
        .create()
        .await?;

    println!("✅ Sandbox created: {}\n", sandbox.id());

    // 1. Python Data Science Example
    println!("1. 🐍 Python Data Science Example");
    let python_result = sandbox.run_python(r#"
import numpy as np
import matplotlib.pyplot as plt
import pandas as pd

# Create sample data
np.random.seed(42)
data = {
    'x': np.random.randn(100),
    'y': np.random.randn(100),
    'category': np.random.choice(['A', 'B', 'C'], 100)
}

df = pd.DataFrame(data)

print("📊 Dataset created:")
print(f"Shape: {df.shape}")
print(f"Categories: {df['category'].value_counts().to_dict()}")

# Statistical summary
print("\n📈 Statistical Summary:")
print(f"Mean X: {df['x'].mean():.3f}")
print(f"Mean Y: {df['y'].mean():.3f}")
print(f"Correlation: {df['x'].corr(df['y']):.3f}")

# Simple analysis
category_means = df.groupby('category')[['x', 'y']].mean()
print(f"\n📋 Category means:\n{category_means}")

"Analysis complete!"
    "#).await?;

    println!("Python Output:");
    println!("{}", python_result.stdout);
    if !python_result.stderr.is_empty() {
        println!("Warnings: {}", python_result.stderr);
    }

    // 2. JavaScript Web Development Example
    println!("\n2. 🟨 JavaScript Web Development Example");
    let js_result = sandbox.run_javascript(r#"
// Simulate a simple web API
const express = require('express');
const fs = require('fs');

// Create mock user data
const users = [
    { id: 1, name: 'Alice', email: 'alice@example.com', role: 'admin' },
    { id: 2, name: 'Bob', email: 'bob@example.com', role: 'user' },
    { id: 3, name: 'Charlie', email: 'charlie@example.com', role: 'user' }
];

console.log('👥 Mock Users Database:');
users.forEach(user => {
    console.log(`  ${user.id}. ${user.name} (${user.role}) - ${user.email}`);
});

// Simulate API operations
function getUsersByRole(role) {
    return users.filter(user => user.role === role);
}

function getUserStats() {
    const roles = users.reduce((acc, user) => {
        acc[user.role] = (acc[user.role] || 0) + 1;
        return acc;
    }, {});
    return roles;
}

console.log('\n📊 User Statistics:');
const stats = getUserStats();
console.log(JSON.stringify(stats, null, 2));

console.log('\n👑 Admin Users:');
const admins = getUsersByRole('admin');
admins.forEach(admin => console.log(`  - ${admin.name}`));

// File operations
const configData = {
    apiVersion: '1.0',
    environment: 'development',
    features: ['authentication', 'logging', 'analytics'],
    timestamp: new Date().toISOString()
};

fs.writeFileSync('/tmp/api_config.json', JSON.stringify(configData, null, 2));
console.log('\n💾 Configuration saved to /tmp/api_config.json');

console.log(`\n🚀 Node.js ${process.version} - API simulation complete!`);
    "#).await?;

    println!("JavaScript Output:");
    println!("{}", js_result.stdout);
    if !js_result.stderr.is_empty() {
        println!("Errors: {}", js_result.stderr);
    }

    // 3. Cross-language data sharing
    println!("\n3. 🔄 Cross-language Data Sharing");

    // Python writes data
    let py_write = sandbox.run_python(r#"
import json

# Python generates data
analysis_results = {
    'model': 'linear_regression',
    'accuracy': 0.95,
    'features': ['feature_1', 'feature_2', 'feature_3'],
    'metadata': {
        'created_by': 'python',
        'timestamp': '2024-01-01T12:00:00Z',
        'version': '1.0'
    }
}

# Save to shared location
with open('/tmp/analysis_results.json', 'w') as f:
    json.dump(analysis_results, f, indent=2)

print("✅ Python: Analysis results saved to /tmp/analysis_results.json")
print(f"Model accuracy: {analysis_results['accuracy']}")
    "#).await?;

    println!("Python data generation:");
    println!("{}", py_write.stdout);

    // JavaScript reads and processes the data
    let js_read = sandbox.run_javascript(r#"
const fs = require('fs');

// Read data created by Python
const analysisData = JSON.parse(fs.readFileSync('/tmp/analysis_results.json', 'utf8'));

console.log('📖 JavaScript: Reading Python-generated data...');
console.log(`Model: ${analysisData.model}`);
console.log(`Accuracy: ${(analysisData.accuracy * 100).toFixed(1)}%`);
console.log(`Features: ${analysisData.features.join(', ')}`);

// JavaScript processes and extends the data
const processedData = {
    ...analysisData,
    processed_by: 'javascript',
    performance_grade: analysisData.accuracy >= 0.9 ? 'Excellent' :
                      analysisData.accuracy >= 0.8 ? 'Good' : 'Needs Improvement',
    recommendations: [
        'Deploy to production',
        'Monitor performance',
        'Collect more training data'
    ]
};

fs.writeFileSync('/tmp/final_report.json', JSON.stringify(processedData, null, 2));
console.log('\n✅ JavaScript: Enhanced report saved to /tmp/final_report.json');
console.log(`Performance Grade: ${processedData.performance_grade}`);
    "#).await?;

    println!("JavaScript data processing:");
    println!("{}", js_read.stdout);

    // 4. Error handling demonstration
    println!("\n4. ⚠️  Error Handling Examples");

    let python_error = sandbox.run_python(r#"
print("This will run successfully")
raise ValueError("This is a deliberate error for demonstration")
print("This line will not be reached")
    "#).await?;

    println!("Python error handling:");
    println!("Stdout: {}", python_error.stdout);
    if let Some(error) = python_error.error {
        println!("Error Type: {}", error.name);
        println!("Error Message: {}", error.value);
        println!("Traceback Available: {}", !error.traceback.is_empty());
    }

    // 5. Language parameter demonstration
    println!("\n5. 🔧 Using Language Parameters");
    let param_result = sandbox.run_code_with_language(
        "console.log('This JavaScript code was executed using the language parameter');",
        "javascript"
    ).await?;
    println!("Language parameter result: {}", param_result.stdout);

    println!("\n🧹 Cleaning up...");
    sandbox.delete().await?;
    println!("✅ Sandbox deleted successfully!");

    println!("\n🎉 Code interpreter demo completed!");
    println!("This demonstrates the power of multi-language execution in E2B sandboxes.");

    Ok(())
}