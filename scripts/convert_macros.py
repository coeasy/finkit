import os
import re
import glob

ROOT = r"P:\llm_code\finkit\core\src\streaming"

SKIP_FILES = {"mod.rs", "types.rs", "traits.rs", "macros.rs", "repaint.rs", 
              "checkpoint.rs", "builder.rs", "registry.rs", "output_types.rs",
              "price_source.rs", "ring_buffer.rs", "rolling_minmax.rs", "float_trait.rs"}

converted = 0
skipped = 0
errors = []

for filepath in sorted(glob.glob(os.path.join(ROOT, "**", "*.rs"), recursive=True)):
    filename = os.path.basename(filepath)
    if filename in SKIP_FILES:
        continue
    
    with open(filepath, "r", encoding="utf-8") as f:
        content = f.read()
    
    # Skip if already converted
    if "impl_standard_methods!" in content:
        skipped += 1
        continue
    
    # Check for standard pattern
    has_count = bool(re.search(r'fn count\(&self\) -> usize \{\s*self\.count\s*\}', content))
    has_value = bool(re.search(r'fn value\(&self\) -> Option<f64> \{\s*self\.last_value\s*\}', content))
    has_meta = "impl IndicatorMeta for" in content
    
    if not (has_count and has_value and has_meta):
        skipped += 1
        continue
    
    original = content
    
    try:
        # Step 1: Replace count() method with impl_standard_methods!()
        # Find the count method and replace it (keeping the value method removal for next step)
        content = re.sub(
            r'(\s+)fn count\(&self\) -> usize \{\s*self\.count\s*\}',
            r'\1impl_standard_methods!();',
            content
        )
        
        # Step 2: Remove the value() method (now part of impl_standard_methods!())
        content = re.sub(
            r'\n\s*fn value\(&self\) -> Option<f64> \{\s*self\.last_value\s*\}\n',
            '\n',
            content
        )
        
        # Step 3: Replace IndicatorMeta impl with macro
        # Match multiline IndicatorMeta impl blocks
        meta_pattern = re.compile(
            r'impl IndicatorMeta for (\w+) \{\s*'
            r'fn name\(\) -> &\'static str \{\s*"([^"]+)"\s*\}\s*'
            r'fn category\(\) -> &\'static str \{\s*"([^"]+)"\s*\}\s*'
            r'fn description\(\) -> &\'static str \{\s*"([^"]+)"\s*\}\s*'
            r'fn warm_up_period\(&self\) -> usize \{\s*self\.period\s*\}\s*\}',
            re.DOTALL
        )
        
        meta_match = meta_pattern.search(content)
        if meta_match:
            type_name = meta_match.group(1)
            name = meta_match.group(2)
            category = meta_match.group(3)
            desc = meta_match.group(4)
            
            replacement = f'impl_indicator_meta!({type_name}, "{name}", "{category}", "{desc}");'
            content = content[:meta_match.start()] + replacement + content[meta_match.end():]
        
        # Step 4: Add imports if needed
        if "impl_standard_methods!" in content or "impl_indicator_meta!" in content:
            imports_needed = []
            if "impl_standard_methods!" in content:
                imports_needed.append("impl_standard_methods")
            if "impl_indicator_meta!" in content:
                imports_needed.append("impl_indicator_meta")
            
            import_line = f"use crate::{{{', '.join(imports_needed)}}};"
            
            if import_line not in content:
                # Find the last use crate::streaming::traits:: line
                last_trait_use = re.search(r'use crate::streaming::traits::\{[^}]+\};', content)
                if last_trait_use:
                    insert_pos = last_trait_use.end()
                    content = content[:insert_pos] + "\n" + import_line + content[insert_pos:]
        
        if content != original:
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(content)
            converted += 1
            print(f"Converted: {os.path.relpath(filepath, ROOT)}")
        else:
            skipped += 1
    
    except Exception as e:
        errors.append(f"Error in {filepath}: {e}")

print(f"\nConverted: {converted}, Skipped: {skipped}, Errors: {len(errors)}")
for err in errors:
    print(err)
