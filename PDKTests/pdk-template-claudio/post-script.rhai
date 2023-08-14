// Copyright 2023 Salesforce, Inc. All rights reserved.
if !variable::get("useconfig") {
	file::delete("src/config.rs")
}

file::rename(".gitignore-template", ".gitignore")
