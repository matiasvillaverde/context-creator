# Edge Case Test Assumptions

This document tracks the assumptions made in edge case tests and their current status.

## Binary File Detection
- **Assumption**: Tool detects and skips binary files
- **Reality**: Tool may process binary files as text
- **Fix**: Either implement binary detection or adjust tests to expect processing

## Line Ending Normalization
- **Assumption**: Tool normalizes line endings to \n
- **Reality**: Tool preserves original line endings
- **Fix**: Adjust tests to check for original line endings

## UTF-8 BOM Handling
- **Assumption**: Tool strips UTF-8 BOM
- **Reality**: Tool preserves BOM
- **Fix**: Adjust tests to allow BOM in output

## Parse Error Detection
- **Assumption**: Tool validates file content matches extension
- **Reality**: Tool processes files regardless of content/extension mismatch
- **Fix**: Adjust tests to expect successful processing

## Semantic Analysis on Single Files
- **Assumption**: Semantic analysis works on individual files
- **Reality**: May require directory context
- **Fix**: Adjust tests to provide directory context or skip if not supported

## Dynamic Import Detection
- **Assumption**: Tool can detect dynamic imports (__import__, getattr)
- **Reality**: Feature not implemented
- **Fix**: Mark tests as known limitations

## Repository Error Messages
- **Assumption**: Specific error messages for branch/auth issues
- **Reality**: Generic error messages
- **Fix**: Check for any error rather than specific text