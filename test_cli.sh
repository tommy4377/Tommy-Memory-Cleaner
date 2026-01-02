#!/bin/bash
echo "Testing Tommy Memory Cleaner CLI..."
echo "=================================="

# Test with WorkingSet
echo "Testing WorkingSet optimization:"
TMC/src-tauri/target/release/TommyMemoryCleaner.exe /WorkingSet &
PID=$!
sleep 2
kill $PID 2>/dev/null
wait $PID 2>/dev/null

echo ""
echo "Testing ModifiedPageList optimization:"
TMC/src-tauri/target/release/TommyMemoryCleaner.exe /ModifiedPageList &
PID=$!
sleep 2
kill $PID 2>/dev/null
wait $PID 2>/dev/null

echo ""
echo "Testing SystemFileCache optimization:"
TMC/src-tauri/target/release/TommyMemoryCleaner.exe /SystemFileCache &
PID=$!
sleep 2
kill $PID 2>/dev/null
wait $PID 2>/dev/null

echo ""
echo "Testing ModifiedFileCache optimization:"
TMC/src-tauri/target/release/TommyMemoryCleaner.exe /ModifiedFileCache &
PID=$!
sleep 2
kill $PID 2>/dev/null
wait $PID 2>/dev/null

echo ""
echo "All tests completed!"
