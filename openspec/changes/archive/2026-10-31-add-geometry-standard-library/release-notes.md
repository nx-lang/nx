# Standard-library source formatting

- NX source now targets 120 characters per line, including trailing property documentation.
  Standalone prose comments remain near 100 characters. Short property descriptions use aligned
  trailing `///` comments with at most two lines; longer explanations use leading blocks.
- Standard-library declaration shapes and behavior are unchanged. Formatting changes affect
  content-derived library versions, so regenerate linked images together.
- `@nx/agent`: property documentation now follows the trailing-comment convention. Its version
  changes from `b06c00c30be50ada` to `0aeeab5b50b2f1fd`; the reference source and SDK expectations
  are updated together.
