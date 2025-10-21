use e2b::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== E2B Code Interpreter Example ===\n");

    let client = Client::new()?;

    println!("Creating sandbox...");
    let sandbox = client
        .sandbox()
        .template("code-interpreter-v1")
        .metadata(json!({
            "example": "code_interpreter",
            "languages": "python,javascript"
        }))
        .timeout(300)
        .create()
        .await?;

    println!("Sandbox created: {}\n", sandbox.id());

    // 1. Python Data Science
    println!("1. Python Data Science Example");
    let python_result = sandbox
        .run_python(
            r#"
import numpy as np
import pandas as pd

np.random.seed(42)
data = {
    'x': np.random.randn(100),
    'y': np.random.randn(100),
    'category': np.random.choice(['A', 'B', 'C'], 100)
}

df = pd.DataFrame(data)

print("Dataset created:")
print(f"Shape: {df.shape}")
print(f"Mean X: {df['x'].mean():.3f}")
print(f"Mean Y: {df['y'].mean():.3f}")

category_means = df.groupby('category')[['x', 'y']].mean()
print(f"\nCategory means:\n{category_means}")
    "#,
        )
        .await?;

    println!("Output:\n{}", python_result.stdout);
    if let Some(err) = python_result.error {
        eprintln!("Error: {} - {}", err.name, err.value);
    }

    // 2. JavaScript Web Development
    println!("\n2. JavaScript Web Development Example");
    let js_result = sandbox
        .run_javascript(
            r#"
const users = [
    { id: 1, name: 'Alice', email: 'alice@example.com', role: 'admin' },
    { id: 2, name: 'Bob', email: 'bob@example.com', role: 'user' },
    { id: 3, name: 'Charlie', email: 'charlie@example.com', role: 'user' }
];

console.log('Users:');
users.forEach(user => {
    console.log(`  ${user.id}. ${user.name} (${user.role})`);
});

const getUserStats = () => {
    return users.reduce((acc, user) => {
        acc[user.role] = (acc[user.role] || 0) + 1;
        return acc;
    }, {});
};

console.log('\nStatistics:');
console.log(JSON.stringify(getUserStats(), null, 2));

console.log(`\nNode.js ${process.version}`);
    "#,
        )
        .await?;

    println!("Output:\n{}", js_result.stdout);
    if let Some(err) = js_result.error {
        eprintln!("Error: {} - {}", err.name, err.value);
    }

    // 3. Cross-language data sharing
    println!("\n3. Cross-language Data Sharing Example");

    // Python writes data
    sandbox
        .run_python(
            r#"
import json

analysis_results = {
    'model': 'linear_regression',
    'accuracy': 0.95,
    'features': ['feature_1', 'feature_2', 'feature_3']
}

with open('/tmp/analysis_results.json', 'w') as f:
    json.dump(analysis_results, f, indent=2)

print("Python: Analysis results saved")
print(f"Model accuracy: {analysis_results['accuracy']}")
    "#,
        )
        .await?;

    // JavaScript reads the data
    let js_read = sandbox
        .run_javascript(
            r#"
const fs = require('fs');

const analysisData = JSON.parse(fs.readFileSync('/tmp/analysis_results.json', 'utf8'));

console.log('JavaScript: Reading Python data...');
console.log(`Model: ${analysisData.model}`);
console.log(`Accuracy: ${(analysisData.accuracy * 100).toFixed(1)}%`);
console.log(`Features: ${analysisData.features.join(', ')}`);
    "#,
        )
        .await?;

    println!("Output:\n{}", js_read.stdout);

    println!("\nCleaning up...");
    sandbox.delete().await?;
    println!("Done!");

    Ok(())
}
