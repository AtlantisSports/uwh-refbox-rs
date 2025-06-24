module led_functional_test(
    input clk,
    input rst_n,
    output bit [7:0] led,
    output bit ls_10a, ls_10b, ls_10c, ls_10d, ls_10e, ls_10f, ls_10g,
    output bit ls_1a, ls_1b, ls_1c, ls_1d, ls_1e, ls_1f, ls_1g,
    output bit rs_10a, rs_10b, rs_10c, rs_10d, rs_10e, rs_10f, rs_10g,
    output bit rs_1a, rs_1b, rs_1c, rs_1d, rs_1e, rs_1f, rs_1g,
    output bit white_on_left, white_on_right,
    output bit m_10a, m_10b, m_10c, m_10d, m_10e, m_10f, m_10g,
    output bit m_1a, m_1b, m_1c, m_1d, m_1e, m_1f, m_1g,
    output bit s_10a, s_10b, s_10c, s_10d, s_10e, s_10f, s_10g,
    output bit s_1a, s_1b, s_1c, s_1d, s_1e, s_1f, s_1g,
    output bit left_to_ind, right_to_ind, ref_to_ind,
    output bit one, slash, two, overtime, sdn_dth,
    output bit colon,
    input usb_rx,
    output bit usb_tx
    );

    wire rst;
    reset_conditioner reset_conditioner(.clk(clk), .in(!rst_n), .out(rst));

    // 1 second timer (assuming 100MHz clock)
    localparam integer CLK_FREQ = 100_000_000;
    reg [26:0] sec_counter = 0;
    reg sec_tick = 0;

    always @(posedge clk or posedge rst) begin
        if (rst) begin
            sec_counter <= 0;
            sec_tick <= 0;
        end else if (sec_counter == CLK_FREQ-1) begin
            sec_counter <= 0;
            sec_tick <= 1;
        end else begin
            sec_counter <= sec_counter + 1;
            sec_tick <= 0;
        end
    end

    // Step counter (0-18)
    reg [4:0] step = 0;
    always @(posedge clk or posedge rst) begin
        if (rst) begin
            step <= 0;
        end else if (sec_tick) begin
            if (step == 19)
                step <= 0;
            else if (
                // Only advance step after current digit_count reaches 9, or for non-digit steps
                (step <= 7 && digit_count == 9) ||
                (step >= 8)
            )
                step <= step + 1;
        end
    end

    // Digit counters for each digit step (0-9)
    reg [3:0] digit_count;
    always @(posedge clk or posedge rst) begin
        if (rst) begin
            digit_count <= 0;
        end else if (sec_tick) begin
            if (step <= 7) begin
                if (digit_count == 9)
                    digit_count <= 0;
                else
                    digit_count <= digit_count + 1;
            end else if (step >= 8) begin
                digit_count <= 0;
            end
        end
    end

    // 7-segment decoder (active high)
    function automatic [6:0] seg7(input [3:0] val);
        case (val)
            4'd0: seg7 = 7'b1111110;
            4'd1: seg7 = 7'b0110000;
            4'd2: seg7 = 7'b1101101;
            4'd3: seg7 = 7'b1111001;
            4'd4: seg7 = 7'b0110011;
            4'd5: seg7 = 7'b1011011;
            4'd6: seg7 = 7'b1011111;
            4'd7: seg7 = 7'b1110000;
            4'd8: seg7 = 7'b1111111;
            4'd9: seg7 = 7'b1111011;
            default: seg7 = 7'b0000000;
        endcase
    endfunction

    // PWM clock divider
    reg [10:0] pwm_counter = 11'b0;
    wire pwm_clk;
    assign pwm_clk = pwm_counter[10];
    always @(posedge clk or posedge rst) begin
        if (rst) begin
            pwm_counter <= 11'b0;
        end else begin
            pwm_counter <= pwm_counter + 1;
        end
    end

    // Spacing 3
    reg [7:0] compare [67:0] = {8'hC9, 8'hC6, 8'hC3, 8'hC0, 8'hBD, 8'hBA, 8'hB7, 8'hB4, 8'hB1, 8'hAE, 8'hAB, 8'hA8, 8'hA5, 8'hA2, 8'h9F, 8'h9C, 8'h99, 8'h96, 8'h93, 8'h90, 8'h8D, 8'h8A, 8'h87, 8'h84, 8'h81, 8'h7E, 8'h7B, 8'h78, 8'h75, 8'h72, 8'h6F, 8'h6C, 8'h69, 8'h66, 8'h63, 8'h60, 8'h5D, 8'h5A, 8'h57, 8'h54, 8'h51, 8'h4E, 8'h4B, 8'h48, 8'h45, 8'h42, 8'h3F, 8'h3C, 8'h39, 8'h36, 8'h33, 8'h30, 8'h2D, 8'h2A, 8'h27, 8'h24, 8'h21, 8'h1E, 8'h1B, 8'h18, 8'h15, 8'h12, 8'h0F, 8'h0C, 8'h09, 8'h06, 8'h03, 8'h00};
    wire [66:0] pwm_out;
    pwm #(.CTR_LEN(8), .NUM_OUTPUTS(67)) pwm(.clk(pwm_clk), .rst(rst), .compare(compare), .pwm(pwm_out));

    // Output enables
    wire ls_10a_en, ls_10b_en, ls_10c_en, ls_10d_en, ls_10e_en, ls_10f_en, ls_10g_en;
    wire ls_1a_en, ls_1b_en, ls_1c_en, ls_1d_en, ls_1e_en, ls_1f_en, ls_1g_en;
    wire rs_10a_en, rs_10b_en, rs_10c_en, rs_10d_en, rs_10e_en, rs_10f_en, rs_10g_en;
    wire rs_1a_en, rs_1b_en, rs_1c_en, rs_1d_en, rs_1e_en, rs_1f_en, rs_1g_en;
    wire m_10a_en, m_10b_en, m_10c_en, m_10d_en, m_10e_en, m_10f_en, m_10g_en;
    wire m_1a_en, m_1b_en, m_1c_en, m_1d_en, m_1e_en, m_1f_en, m_1g_en;
    wire s_10a_en, s_10b_en, s_10c_en, s_10d_en, s_10e_en, s_10f_en, s_10g_en;
    wire s_1a_en, s_1b_en, s_1c_en, s_1d_en, s_1e_en, s_1f_en, s_1g_en;
    wire white_on_left_en, white_on_right_en;
    wire left_to_ind_en, right_to_ind_en, ref_to_ind_en;
    wire one_en, slash_en, two_en, overtime_en, sdn_dth_en;
    wire colon_en;

    wire [6:0] seg7_digit;
    assign seg7_digit = seg7(digit_count);

    // Set enables based on step and digit segment
    assign ls_10a_en = (step == 0) && seg7_digit[6];
    assign ls_10b_en = (step == 0) && seg7_digit[5];
    assign ls_10c_en = (step == 0) && seg7_digit[4];
    assign ls_10d_en = (step == 0) && seg7_digit[3];
    assign ls_10e_en = (step == 0) && seg7_digit[2];
    assign ls_10f_en = (step == 0) && seg7_digit[1];
    assign ls_10g_en = (step == 0) && seg7_digit[0];

    assign ls_1a_en = (step == 1) && seg7_digit[6];
    assign ls_1b_en = (step == 1) && seg7_digit[5];
    assign ls_1c_en = (step == 1) && seg7_digit[4];
    assign ls_1d_en = (step == 1) && seg7_digit[3];
    assign ls_1e_en = (step == 1) && seg7_digit[2];
    assign ls_1f_en = (step == 1) && seg7_digit[1];
    assign ls_1g_en = (step == 1) && seg7_digit[0];

    assign m_10a_en = (step == 2) && seg7_digit[6];
    assign m_10b_en = (step == 2) && seg7_digit[5];
    assign m_10c_en = (step == 2) && seg7_digit[4];
    assign m_10d_en = (step == 2) && seg7_digit[3];
    assign m_10e_en = (step == 2) && seg7_digit[2];
    assign m_10f_en = (step == 2) && seg7_digit[1];
    assign m_10g_en = (step == 2) && seg7_digit[0];

    assign m_1a_en = (step == 3) && seg7_digit[6];
    assign m_1b_en = (step == 3) && seg7_digit[5];
    assign m_1c_en = (step == 3) && seg7_digit[4];
    assign m_1d_en = (step == 3) && seg7_digit[3];
    assign m_1e_en = (step == 3) && seg7_digit[2];
    assign m_1f_en = (step == 3) && seg7_digit[1];
    assign m_1g_en = (step == 3) && seg7_digit[0];

    assign s_10a_en = (step == 4) && seg7_digit[6];
    assign s_10b_en = (step == 4) && seg7_digit[5];
    assign s_10c_en = (step == 4) && seg7_digit[4];
    assign s_10d_en = (step == 4) && seg7_digit[3];
    assign s_10e_en = (step == 4) && seg7_digit[2];
    assign s_10f_en = (step == 4) && seg7_digit[1];
    assign s_10g_en = (step == 4) && seg7_digit[0];

    assign s_1a_en = (step == 5) && seg7_digit[6];
    assign s_1b_en = (step == 5) && seg7_digit[5];
    assign s_1c_en = (step == 5) && seg7_digit[4];
    assign s_1d_en = (step == 5) && seg7_digit[3];
    assign s_1e_en = (step == 5) && seg7_digit[2];
    assign s_1f_en = (step == 5) && seg7_digit[1];
    assign s_1g_en = (step == 5) && seg7_digit[0];

    assign rs_10a_en = (step == 6) && seg7_digit[6];
    assign rs_10b_en = (step == 6) && seg7_digit[5];
    assign rs_10c_en = (step == 6) && seg7_digit[4];
    assign rs_10d_en = (step == 6) && seg7_digit[3];
    assign rs_10e_en = (step == 6) && seg7_digit[2];
    assign rs_10f_en = (step == 6) && seg7_digit[1];
    assign rs_10g_en = (step == 6) && seg7_digit[0];

    assign rs_1a_en = (step == 7) && seg7_digit[6];
    assign rs_1b_en = (step == 7) && seg7_digit[5];
    assign rs_1c_en = (step == 7) && seg7_digit[4];
    assign rs_1d_en = (step == 7) && seg7_digit[3];
    assign rs_1e_en = (step == 7) && seg7_digit[2];
    assign rs_1f_en = (step == 7) && seg7_digit[1];
    assign rs_1g_en = (step == 7) && seg7_digit[0];

    assign colon_en = (step == 8);
    assign white_on_left_en = (step == 9);
    assign white_on_right_en = (step == 10);
    assign left_to_ind_en = (step == 11);
    assign ref_to_ind_en = (step == 12);
    assign right_to_ind_en = (step == 13);
    assign one_en = (step == 14);
    assign slash_en = (step == 15);
    assign two_en = (step == 16);
    assign overtime_en = (step == 17);
    assign sdn_dth_en = (step == 18);

    // Assign outputs from pwm_out, gated by enables
    assign ls_10a = pwm_out[0] & ls_10a_en;
    assign ls_10b = pwm_out[1] & ls_10b_en;
    assign ls_10c = pwm_out[2] & ls_10c_en;
    assign ls_10d = pwm_out[3] & ls_10d_en;
    assign ls_10e = pwm_out[4] & ls_10e_en;
    assign ls_10f = pwm_out[5] & ls_10f_en;
    assign ls_10g = pwm_out[6] & ls_10g_en;

    assign ls_1a = pwm_out[7] & ls_1a_en;
    assign ls_1b = pwm_out[8] & ls_1b_en;
    assign ls_1c = pwm_out[9] & ls_1c_en;
    assign ls_1d = pwm_out[10] & ls_1d_en;
    assign ls_1e = pwm_out[11] & ls_1e_en;
    assign ls_1f = pwm_out[12] & ls_1f_en;
    assign ls_1g = pwm_out[13] & ls_1g_en;

    assign rs_10a = pwm_out[14] & rs_10a_en;
    assign rs_10b = pwm_out[15] & rs_10b_en;
    assign rs_10c = pwm_out[16] & rs_10c_en;
    assign rs_10d = pwm_out[17] & rs_10d_en;
    assign rs_10e = pwm_out[18] & rs_10e_en;
    assign rs_10f = pwm_out[19] & rs_10f_en;
    assign rs_10g = pwm_out[20] & rs_10g_en;

    assign rs_1a = pwm_out[21] & rs_1a_en;
    assign rs_1b = pwm_out[22] & rs_1b_en;
    assign rs_1c = pwm_out[23] & rs_1c_en;
    assign rs_1d = pwm_out[24] & rs_1d_en;
    assign rs_1e = pwm_out[25] & rs_1e_en;
    assign rs_1f = pwm_out[26] & rs_1f_en;
    assign rs_1g = pwm_out[27] & rs_1g_en;

    assign white_on_left = pwm_out[28] & white_on_left_en;
    assign white_on_right = pwm_out[29] & white_on_right_en;

    assign m_10a = pwm_out[30] & m_10a_en;
    assign m_10b = pwm_out[31] & m_10b_en;
    assign m_10c = pwm_out[32] & m_10c_en;
    assign m_10d = pwm_out[33] & m_10d_en;
    assign m_10e = pwm_out[34] & m_10e_en;
    assign m_10f = pwm_out[35] & m_10f_en;
    assign m_10g = pwm_out[36] & m_10g_en;

    assign m_1a = pwm_out[37] & m_1a_en;
    assign m_1b = pwm_out[38] & m_1b_en;
    assign m_1c = pwm_out[39] & m_1c_en;
    assign m_1d = pwm_out[40] & m_1d_en;
    assign m_1e = pwm_out[41] & m_1e_en;
    assign m_1f = pwm_out[42] & m_1f_en;
    assign m_1g = pwm_out[43] & m_1g_en;

    assign s_10a = pwm_out[44] & s_10a_en;
    assign s_10b = pwm_out[45] & s_10b_en;
    assign s_10c = pwm_out[46] & s_10c_en;
    assign s_10d = pwm_out[47] & s_10d_en;
    assign s_10e = pwm_out[48] & s_10e_en;
    assign s_10f = pwm_out[49] & s_10f_en;
    assign s_10g = pwm_out[50] & s_10g_en;

    assign s_1a = pwm_out[51] & s_1a_en;
    assign s_1b = pwm_out[52] & s_1b_en;
    assign s_1c = pwm_out[53] & s_1c_en;
    assign s_1d = pwm_out[54] & s_1d_en;
    assign s_1e = pwm_out[55] & s_1e_en;
    assign s_1f = pwm_out[56] & s_1f_en;
    assign s_1g = pwm_out[57] & s_1g_en;

    assign left_to_ind = pwm_out[58] & left_to_ind_en;
    assign right_to_ind = pwm_out[59] & right_to_ind_en;
    assign ref_to_ind = pwm_out[60] & ref_to_ind_en;

    assign one = pwm_out[61] & one_en;
    assign slash = pwm_out[62] & slash_en;
    assign two = pwm_out[63] & two_en;
    assign overtime = pwm_out[64] & overtime_en;
    assign sdn_dth = pwm_out[65] & sdn_dth_en;

    assign colon = pwm_out[66] & colon_en;

    assign led[7:0] = {3'b0, step[4:0]};

    assign usb_tx = 0;

endmodule
