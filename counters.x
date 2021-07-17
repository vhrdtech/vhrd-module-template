SECTIONS
{
  .counters 0 (INFO) :
  {
    /* Format implementations for primitives like u8 */
    *(.counters.*);

    __COUNTERS_MARKER_END = .;
  }
}

ASSERT(__COUNTERS_MARKER_END < 65534, "Maximum amount of counters supported is 65534");