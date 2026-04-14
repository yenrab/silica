#!/bin/bash
echo "| Test File | Actual Exit Code | Expected | Status |"
echo "|-----------|------------------|----------|--------|"

for silica_file in *.silica; do
    base=$(basename "$silica_file" .silica)
    if [ -f "$base" ]; then
        ./$base >/dev/null 2>&1
        actual=$?
        
        # Determine expected value
        expected=42  # Default
        
        case "$base" in
            "tuple_decomp_example")
                expected=50  # 306 % 256
                ;;
            "test_tuple_int")
                expected=2   # y from (1, 2)
                ;;
            "simple_tuple_test")
                expected=42  # x from (42, true)
                ;;
        esac
        
        if [ $actual -eq $expected ]; then
            status="✅ PASS"
        else
            status="❌ FAIL"
        fi
        
        printf "| %-25s | %-16s | %-8s | %s |\n" "$base" "$actual" "$expected" "$status"
    fi
done
