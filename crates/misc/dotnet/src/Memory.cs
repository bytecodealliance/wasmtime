using System;
using System.Text;
using System.Buffers.Binary;

namespace Wasmtime
{
    /// <summary>
    /// Represents a WebAssembly memory.
    /// </summary>
    public class Memory
    {
        /// <summary>
        /// The size, in bytes, of a WebAssembly memory page.
        /// </summary>
        public const int PageSize = 65536;

        /// <summary>
        /// Creates a new memory with the given minimum and maximum page counts.
        /// </summary>
        /// <param name="minimum"></param>
        /// <param name="maximum"></param>
        public Memory(uint minimum = 1, uint maximum = uint.MaxValue)
        {
            if (minimum == 0)
            {
                throw new ArgumentException("The minimum cannot be zero..", nameof(minimum));
            }

            if (maximum < minimum)
            {
                throw new ArgumentException("The maximum cannot be less than the minimum.", nameof(maximum));
            }

            Minimum = minimum;
            Maximum = maximum;
        }

        /// <summary>
        /// The minimum memory size (in WebAssembly page units).
        /// </summary>
        public uint Minimum { get; private set; }

        /// <summary>
        /// The minimum memory size (in WebAssembly page units).
        /// </summary>
        public uint Maximum { get; private set; }

        /// <summary>
        /// The span of the memory.
        /// </summary>
        /// <remarks>
        /// The span may become invalid if the memory grows.
        ///
        /// This may happen if the memory is explicitly requested to grow or
        /// grows as a result of WebAssembly execution.
        ///
        /// Therefore, the returned Span should not be stored.
        /// </remarks>
        public unsafe Span<byte> Span
        {
            get
            {
                var data = Interop.wasm_memory_data(_handle.DangerousGetHandle());
                var size = Convert.ToInt32(Interop.wasm_memory_data_size(_handle.DangerousGetHandle()).ToUInt32());
                return new Span<byte>(data, size);
            }
        }

        /// <summary>
        /// Reads a UTF-8 string from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <param name="length">The length of bytes to read.</param>
        /// <returns>Returns the string read from memory.</returns>
        public string ReadString(int address, int length)
        {
            return Encoding.UTF8.GetString(Span.Slice(address, length));
        }

        /// <summary>
        /// Writes a UTF-8 string at the given address.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The string to write.</param>
        /// <return>Returns the number of bytes written.</return>
        public int WriteString(int address, string value)
        {
            return Encoding.UTF8.GetBytes(value, Span.Slice(address));
        }

        /// <summary>
        /// Reads a byte from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <returns>Returns the byte read from memory.</returns>
        public byte ReadByte(int address)
        {
            return Span[address];
        }

        /// <summary>
        /// Writes a byte to memory.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The byte to write.</param>
        public void WriteByte(int address, byte value)
        {
            Span[address] = value;
        }

        /// <summary>
        /// Reads a short from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <returns>Returns the short read from memory.</returns>
        public short ReadInt16(int address)
        {
            return BinaryPrimitives.ReadInt16LittleEndian(Span.Slice(address, 2));
        }

        /// <summary>
        /// Writes a short to memory.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The short to write.</param>
        public void WriteInt16(int address, short value)
        {
            BinaryPrimitives.WriteInt16LittleEndian(Span.Slice(address, 2), value);
        }

        /// <summary>
        /// Reads an int from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <returns>Returns the int read from memory.</returns>
        public int ReadInt32(int address)
        {
            return BinaryPrimitives.ReadInt32LittleEndian(Span.Slice(address, 4));
        }

        /// <summary>
        /// Writes an int to memory.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The int to write.</param>
        public void WriteInt32(int address, int value)
        {
            BinaryPrimitives.WriteInt32LittleEndian(Span.Slice(address, 4), value);
        }

        /// <summary>
        /// Reads a long from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <returns>Returns the long read from memory.</returns>
        public long ReadInt64(int address)
        {
            return BinaryPrimitives.ReadInt64LittleEndian(Span.Slice(address, 8));
        }

        /// <summary>
        /// Writes a long to memory.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The long to write.</param>
        public void WriteInt64(int address, long value)
        {
            BinaryPrimitives.WriteInt64LittleEndian(Span.Slice(address, 8), value);
        }

        /// <summary>
        /// Reads an IntPtr from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <returns>Returns the IntPtr read from memory.</returns>
        public IntPtr ReadIntPtr(int address)
        {
            if (IntPtr.Size == 4)
            {
                return (IntPtr)ReadInt32(address);
            }
            return (IntPtr)ReadInt64(address);
        }

        /// <summary>
        /// Writes an IntPtr to memory.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The IntPtr to write.</param>
        public void WriteIntPtr(int address, IntPtr value)
        {
            if (IntPtr.Size == 4)
            {
                WriteInt32(address, value.ToInt32());
            }
            else
            {
                WriteInt64(address, value.ToInt64());
            }
        }

        /// <summary>
        /// Reads a long from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <returns>Returns the long read from memory.</returns>
        public float ReadSingle(int address)
        {
            unsafe
            {
                var i = ReadInt32(address);
                return *((float*)&i);
            }
        }

        /// <summary>
        /// Writes a single to memory.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The single to write.</param>
        public void WriteSingle(int address, float value)
        {
            unsafe
            {
                WriteInt32(address, *(int*)&value);
            }
        }

        /// <summary>
        /// Reads a double from memory.
        /// </summary>
        /// <param name="address">The zero-based address to read from.</param>
        /// <returns>Returns the double read from memory.</returns>
        public double ReadDouble(int address)
        {
            unsafe
            {
                var i = ReadInt64(address);
                return *((double*)&i);
            }
        }

        /// <summary>
        /// Writes a double to memory.
        /// </summary>
        /// <param name="address">The zero-based address to write to.</param>
        /// <param name="value">The double to write.</param>
        public void WriteDouble(int address, double value)
        {
            unsafe
            {
                WriteInt64(address, *(long*)&value);
            }
        }

        internal Interop.MemoryHandle Handle
        {
            get
            {
                return _handle;
            }
            set
            {
                _handle = value;
            }
        }

        private Interop.MemoryHandle _handle;
    }
}
