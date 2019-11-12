// Part of the Wasmtime Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://github.com/bytecodealliance/wasmtime/blob/master/LICENSE for license information.
//
// Significant parts of this file are derived from cloudabi-utils. See
// https://github.com/bytecodealliance/wasmtime/blob/master/lib/wasi/sandboxed-system-primitives/src/LICENSE
// for license information.
//
// The upstream file contains the following copyright notice:
//
// Copyright (c) 2016 Nuxi, https://nuxi.nl/

#ifndef QUEUE_H
#define QUEUE_H

#include <stddef.h>

// LIST: Double-linked list.

#define LIST_HEAD(name, type) \
  struct name {               \
    struct type *l_first;     \
  }
#define LIST_HEAD_INITIALIZER(head) \
  { NULL }

#define LIST_ENTRY(type)  \
  struct {                \
    struct type *l_next;  \
    struct type **l_prev; \
  }

#define LIST_FOREACH(var, head, field) \
  for ((var) = (head)->l_first; (var) != NULL; (var) = (var)->field.l_next)
#define LIST_INIT(head)     \
  do {                      \
    (head)->l_first = NULL; \
  } while (0)
#define LIST_INSERT_HEAD(head, element, field)                  \
  do {                                                          \
    (element)->field.l_next = (head)->l_first;                  \
    if ((head)->l_first != NULL)                                \
      (head)->l_first->field.l_prev = &(element)->field.l_next; \
    (head)->l_first = (element);                                \
    (element)->field.l_prev = &(head)->l_first;                 \
  } while (0)
#define LIST_REMOVE(element, field)                                    \
  do {                                                                 \
    if ((element)->field.l_next != NULL)                               \
      (element)->field.l_next->field.l_prev = (element)->field.l_prev; \
    *(element)->field.l_prev = (element)->field.l_next;                \
  } while (0)

// TAILQ: Double-linked list with tail pointer.

#define TAILQ_HEAD(name, type) \
  struct name {                \
    struct type *t_first;      \
    struct type **t_last;      \
  }

#define TAILQ_ENTRY(type) \
  struct {                \
    struct type *t_next;  \
    struct type **t_prev; \
  }

#define TAILQ_EMPTY(head) ((head)->t_first == NULL)
#define TAILQ_FIRST(head) ((head)->t_first)
#define TAILQ_FOREACH(var, head, field) \
  for ((var) = (head)->t_first; (var) != NULL; (var) = (var)->field.t_next)
#define TAILQ_INIT(head)               \
  do {                                 \
    (head)->t_first = NULL;            \
    (head)->t_last = &(head)->t_first; \
  } while (0)
#define TAILQ_INSERT_TAIL(head, elm, field) \
  do {                                      \
    (elm)->field.t_next = NULL;             \
    (elm)->field.t_prev = (head)->t_last;   \
    *(head)->t_last = (elm);                \
    (head)->t_last = &(elm)->field.t_next;  \
  } while (0)
#define TAILQ_REMOVE(head, element, field)                             \
  do {                                                                 \
    if ((element)->field.t_next != NULL)                               \
      (element)->field.t_next->field.t_prev = (element)->field.t_prev; \
    else                                                               \
      (head)->t_last = (element)->field.t_prev;                        \
    *(element)->field.t_prev = (element)->field.t_next;                \
  } while (0)

#endif
