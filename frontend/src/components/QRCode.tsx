import { useEffect, useRef, useState } from 'react'
import qrcode from 'qrcode-generator'

interface QRCodeProps {
  value: string
  size?: number
}

export default function QRCode({ value, size = 200 }: QRCodeProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    if (!canvasRef.current || !value) return

    try {
      const qr = qrcode(0, 'M')
      qr.addData(value)
      qr.make()

      const canvas = canvasRef.current
      const ctx = canvas.getContext('2d')
      if (!ctx) return

      const moduleCount = qr.getModuleCount()
      const cellSize = Math.floor(size / moduleCount)
      const canvasSize = cellSize * moduleCount

      canvas.width = canvasSize
      canvas.height = canvasSize

      ctx.fillStyle = '#ffffff'
      ctx.fillRect(0, 0, canvasSize, canvasSize)

      ctx.fillStyle = '#000000'
      for (let row = 0; row < moduleCount; row++) {
        for (let col = 0; col < moduleCount; col++) {
          if (qr.isDark(row, col)) {
            ctx.fillRect(col * cellSize, row * cellSize, cellSize, cellSize)
          }
        }
      }
      setError(null)
    } catch (e) {
      setError('Failed to generate QR code')
      console.error('QR code generation error:', e)
    }
  }, [value, size])

  if (error) {
    return (
      <div className="text-red-400 text-sm font-mono p-4 text-center">
        {error}
      </div>
    )
  }

  return (
    <canvas
      ref={canvasRef}
      className="rounded-lg mx-auto"
      style={{ width: size, height: size, imageRendering: 'pixelated' }}
    />
  )
}
