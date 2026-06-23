# Shared transitive lib/*.o resolution for stdlib/data_structures trial linking.
# Include after THIS_DIR is set (same discovery pattern as stdlib_prereq.mk).
# Requires shell variable: base (trial basename without .silica, e.g. btree_nodeid_empty_get)

define COLLECT_LIB_OBJS
lib_objs=""; \
for mod in $$(grep '^use ' "$$base.silica" 2>/dev/null | sed 's/^use //;s/;//'); do \
	lib_obj="lib/$$mod.o"; \
	[ -f "$$lib_obj" ] && lib_objs="$$lib_objs $$lib_obj"; \
done; \
case "$$lib_objs" in \
	*lib/btree_set_nodeid.o*) \
		case "$$lib_objs" in \
			*lib/btree_set_csr.o*) ;; \
			*) [ -f lib/btree_set_csr.o ] && lib_objs="$$lib_objs lib/btree_set_csr.o" ;; \
		esac ;; \
esac; \
case "$$lib_objs" in \
	*lib/btree_nodeid.o*) \
		case "$$lib_objs" in \
			*lib/btree_csr_map.o*) ;; \
			*) [ -f lib/btree_csr_map.o ] && lib_objs="$$lib_objs lib/btree_csr_map.o" ;; \
		esac ;; \
esac; \
case "$$lib_objs" in \
	*lib/OrderedSet.o*) \
		case "$$lib_objs" in \
			*lib/ordered_set_nodeid_adapter.o*) ;; \
			*) [ -f lib/ordered_set_nodeid_adapter.o ] && lib_objs="$$lib_objs lib/ordered_set_nodeid_adapter.o" ;; \
		esac; \
		case "$$lib_objs" in \
			*lib/ordered_set_csr_adapter.o*) ;; \
			*) [ -f lib/ordered_set_csr_adapter.o ] && lib_objs="$$lib_objs lib/ordered_set_csr_adapter.o" ;; \
		esac; \
		case "$$lib_objs" in \
			*lib/btree_set_nodeid.o*) ;; \
			*) [ -f lib/btree_set_nodeid.o ] && lib_objs="$$lib_objs lib/btree_set_nodeid.o" ;; \
		esac; \
		case "$$lib_objs" in \
			*lib/btree_set_csr.o*) ;; \
			*) [ -f lib/btree_set_csr.o ] && lib_objs="$$lib_objs lib/btree_set_csr.o" ;; \
		esac ;; \
esac; \
case "$$lib_objs" in \
	*lib/OrderedMap.o*) \
		case "$$lib_objs" in \
			*lib/ordered_map_nodeid_adapter.o*) ;; \
			*) [ -f lib/ordered_map_nodeid_adapter.o ] && lib_objs="$$lib_objs lib/ordered_map_nodeid_adapter.o" ;; \
		esac; \
		case "$$lib_objs" in \
			*lib/ordered_map_csr_adapter.o*) ;; \
			*) [ -f lib/ordered_map_csr_adapter.o ] && lib_objs="$$lib_objs lib/ordered_map_csr_adapter.o" ;; \
		esac; \
		case "$$lib_objs" in \
			*lib/btree_nodeid.o*) ;; \
			*) [ -f lib/btree_nodeid.o ] && lib_objs="$$lib_objs lib/btree_nodeid.o" ;; \
		esac; \
		case "$$lib_objs" in \
			*lib/btree_csr_map.o*) ;; \
			*) [ -f lib/btree_csr_map.o ] && lib_objs="$$lib_objs lib/btree_csr_map.o" ;; \
		esac ;; \
esac; \
case "$$lib_objs" in \
	*lib/csr_set_trait_bridge.o*) \
		case "$$lib_objs" in \
			*lib/btree_set_csr.o*) ;; \
			*) [ -f lib/btree_set_csr.o ] && lib_objs="$$lib_objs lib/btree_set_csr.o" ;; \
		esac ;; \
esac; \
case "$$lib_objs" in \
	*lib/csr_map_trait_bridge.o*) \
		case "$$lib_objs" in \
			*lib/btree_csr_map.o*) ;; \
			*) [ -f lib/btree_csr_map.o ] && lib_objs="$$lib_objs lib/btree_csr_map.o" ;; \
		esac ;; \
esac; \
case "$$lib_objs" in \
	*lib/graph_adj_directed.o*|*lib/graph_adj_undirected.o*|*lib/graph_dense_directed.o*|*lib/graph_dense_bitset_directed.o*|*lib/graph_csr_directed.o*) \
		case "$$lib_objs" in \
			*lib/graph_adj_list_helpers.o*) ;; \
			*) [ -f lib/graph_adj_list_helpers.o ] && lib_objs="$$lib_objs lib/graph_adj_list_helpers.o" ;; \
		esac ;; \
esac; \
case "$$lib_objs" in \
	*lib/graph_csr_directed.o*) \
		case "$$lib_objs" in \
			*lib/graph_adj_directed.o*) ;; \
			*) [ -f lib/graph_adj_directed.o ] && lib_objs="$$lib_objs lib/graph_adj_directed.o" ;; \
		esac ;; \
esac
endef
