#pragma once

#ifndef OSPF_BASE_MULTI_THREAD
#ifdef OSPF_MULTI_THREAD
#define OSPF_BASE_MULTI_THREAD on
#else
#define OSPF_BASE_MULTI_THREAD off
#endif
#endif
