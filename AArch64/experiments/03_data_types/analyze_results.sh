#!/bin/bash
echo "| Test File | Actual Exit Code | Expected | Status | Description |"
echo "|-----------|------------------|----------|--------|-------------|"

# Function to get expected value and description
get_expected() {
    case "$1" in
        "tuple_decomp_example")
            echo "50|Complex tuple decomposition (300+1+2+3=306, 306%256=50)"
            ;;
        "test_tuple_int")
            echo "2|Returns y from tuple (1,2)"
            ;;
        "simple_tuple_decomp")
            echo "75|Sum of coordinates (5+10+10+20+30=75)"
            ;;
        "simple_type_alias")
            echo "84|Sum of two results (42+42=84)"
            ;;
        "test_paren_expr")
            echo "3|Result of (1+2)"
            ;;
        "test_struct_basic")
            echo "10|Point.x field access"
            ;;
        "test_struct_literal")
            echo "100|Complex struct operations"
            ;;
        "test_struct_simple")
            echo "30|Point.x + Point.y (10+20=30)"
            ;;
        "test_sum_type")
            echo "66|Sum type operations"
            ;;
        "test_tuple")
            echo "44|Tuple operations"
            ;;
        "test_tuple_bind")
            echo "1|Tuple binding result"
            ;;
        "test_tuple_case_simple")
            echo "100|Tuple case matching"
            ;;
        "test_tuple_literal")
            echo "44|Tuple literal operations"
            ;;
        "tuple_decomposition_example")
            echo "115|Complex decomposition example"
            ;;
        "tuple_types_test")
            echo "48|Tuple type testing"
            ;;
        "working_tuple_decomp")
            echo "72|Working tuple decomposition"
            ;;
        *)
            echo "42|Standard test return value"
            ;;
    esac
}

while read line; do
    if [[ $line =~ \|.*\|.*\|.*\|.*\| ]]; then
        test_file=$(echo "$line" | awk -F'|' '{print $2}' | xargs)
        actual=$(echo "$line" | awk -F'|' '{print $3}' | xargs)
        
        if [ -n "$test_file" ] && [ "$test_file" != "Test File" ]; then
            expected_desc=$(get_expected "$test_file")
            expected=$(echo "$expected_desc" | cut -d'|' -f1)
            desc=$(echo "$expected_desc" | cut -d'|' -f2)
            
            if [ "$actual" = "$expected" ]; then
                status="✅ PASS"
            else
                status="❌ FAIL"
            fi
            
            printf "| %-25s | %-16s | %-8s | %-6s | %s |\n" "$test_file" "$actual" "$expected" "$status" "$desc"
        fi
    fi
done < results.txt
