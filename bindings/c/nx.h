#pragma once

#include <stddef.h>
#include <stdint.h>

#ifdef _WIN32
#define NX_FFI_EXPORT __declspec(dllimport)
#else
#define NX_FFI_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef struct NxBuffer
{
    uint8_t* ptr;
    size_t len;
    size_t cap;
} NxBuffer;

typedef enum NxEvalStatus
{
    NxEvalStatus_Ok = 0,
    NxEvalStatus_Error = 1,
    NxEvalStatus_InvalidArgument = 2,
    NxEvalStatus_Panic = 255,
} NxEvalStatus;

NX_FFI_EXPORT NxEvalStatus nx_eval_source_msgpack(
    const uint8_t* source_ptr,
    size_t source_len,
    const uint8_t* file_name_ptr,
    size_t file_name_len,
    NxBuffer* out_buffer);

NX_FFI_EXPORT NxEvalStatus nx_eval_source_json(
    const uint8_t* source_ptr,
    size_t source_len,
    const uint8_t* file_name_ptr,
    size_t file_name_len,
    NxBuffer* out_buffer);

NX_FFI_EXPORT void nx_free_buffer(NxBuffer buffer);

#ifdef __cplusplus
}
#endif

